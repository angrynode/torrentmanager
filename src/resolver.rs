use hightorrent_api::hightorrent::{MagnetLink, TorrentFile, TorrentID};
use librqbit::*;
use sea_orm::*;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

use std::collections::HashMap;
use std::sync::Arc;

use crate::database::operator::{DatabaseOperator, TableOperator};
use crate::database::torrent::{
    ActiveModel as TorrentActiveModel, Model as TorrentModel, TorrentOperation,
};
use crate::state::AppState;

#[derive(Clone, Debug)]
pub enum ResolverAction {
    /// Resolve the magnet link, then save the torrent file.
    Resolve(MagnetLink),
    /// Stop resolving the magnet link, because the corresponding
    /// torrent file was uploaded manually.
    Cancel(TorrentID),
}

/// A magnet link resolver
///
/// The resolver keeps track of background tasks (JoinHandle) and
/// listens to new messages for new tasks or cancelations.
pub struct Resolver {
    operator: DatabaseOperator,
    // Channel to receive new magnets to resolve
    receiver: UnboundedReceiver<ResolverAction>,
    /// The rqbit session
    session: Arc<Session>,
    // Keep track of background tasks resolving torrent files from magnets
    // so then can be polled/canceled.
    tasks: HashMap<TorrentID, JoinHandle<TorrentFile>>,
}

impl Resolver {
    /// Initialize rqbit in the background
    pub async fn new(state: AppState, receiver: UnboundedReceiver<ResolverAction>) -> Self {
        let session = Session::new_with_opts(
            state.config.rqbit_path.clone().into(),
            SessionOptions {
                listen: Some(ListenerOptions::default()),
                ..Default::default()
            },
        )
        // TODO: catch errors here? Why would it fail though?
        .await
        .unwrap();

        Self {
            operator: DatabaseOperator::new(state, None),
            receiver,
            session,
            tasks: HashMap::new(),
        }
    }

    /// Starts a background task to resolve the magnet
    ///
    /// If the magnet is already being resolved, this is a no-op.
    pub fn resolve(&mut self, magnet: MagnetLink) {
        if self.tasks.contains_key(&magnet.id()) {
            log::warn!(
                "Magnet {} is already resolving. This is a logic bug.",
                magnet.id()
            );
            return;
        }

        log::info!("Starting to resolve magnet {}", magnet.id());
        let session = self.session.clone();
        let magnet_str = magnet.to_string();
        let handle = tokio::task::spawn(async move {
            let resp = session
                .add_torrent(
                    AddTorrent::from_url(magnet_str),
                    Some(AddTorrentOptions {
                        list_only: true,
                        ..Default::default()
                    }),
                )
                .await
                .unwrap();

            match resp {
                AddTorrentResponse::ListOnly(resp) => {
                    log::info!("Found metainfo for torrent {:?}", resp.info_hash);
                    TorrentFile::from_slice(&resp.torrent_bytes).unwrap()
                    // TODO: here we should process the resolved torrent
                }
                _ => {
                    panic!("RQBIT BUG!");
                }
            }
        });

        self.tasks.insert(magnet.id(), handle);
    }

    /// Periodically check if new magnets have been added. If so,
    /// start a new task to resolve the magnet.
    ///
    /// When the magnet is resolved, save the new value in the DB.
    pub async fn serve(&mut self) {
        log::info!("Starting the magnet resolver. Checking for saved unresolved magnets.");
        let unresolved_torrents: Vec<TorrentModel> = self
            .operator
            .torrent()
            .list()
            .await
            .unwrap()
            .into_iter()
            .filter(|x| x.torrent_file.is_none())
            .collect();
        for torrent in unresolved_torrents {
            // Start a new resolving task
            self.resolve(torrent.magnet_link);
        }

        // Now keep looking for new magnets to resolve
        loop {
            // First, let's check if we have successfully resolved some magnets
            // to further process them (TODO).
            self.save_resolved().await;

            // We apply a one-second timeout so we can go back to save_resolved
            // if there are no new submitted magnets. UnboundedReceiver::recv is
            // cancel-safe so no message will be dropped here. But just to avoid
            // shenanigans we use a proper sleep and not a timeout of the
            // actual receiver.
            tokio::select! {
                _ = sleep(Duration::from_secs(1)) => {},
                recv = self.receiver.recv() => {
                    match recv.expect("magnetlink resolver sender has been closed") {
                        ResolverAction::Resolve(magnet_link) => self.resolve(magnet_link),
                        ResolverAction::Cancel(torrent_id) => self.cancel(&torrent_id),
                    }
                }
            };
        }
    }

    /// Returns the list of TorrentIDs whose resolution tasks have finished
    fn finished_ids(&self) -> Vec<TorrentID> {
        self.tasks
            .iter()
            .filter_map(|(k, v)| {
                if v.is_finished() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Cancels resolution for a given TorrentID.
    ///
    /// Does not produce an error if there was no resolution ongoing.
    fn cancel(&mut self, id: &TorrentID) {
        if self.tasks.remove(id).is_some() {
            log::info!("Canceled resolution for magnet {id}");
        }
    }

    /// Check if some tasks have finished resolving, and save result in the DB.
    pub async fn save_resolved(&mut self) {
        for finished_id in self.finished_ids() {
            log::info!("Magnet {finished_id} has finished resolving. Saving to DB.");

            // Get the raw task handle, removing it from the active tasks
            let handle = self.tasks.remove(&finished_id).unwrap();
            let torrent_file = handle.await.unwrap();

            // If the torrent was deleted for some reason, this is a no-op
            if let Ok(torrent) = self
                .operator
                .torrent()
                .get_by_torrent_id(&finished_id)
                .await
            {
                if torrent.torrent_file.is_some() {
                    log::info!(
                        "Magnet {finished_id} already has a torrent file in DB. Maybe it was uploaded manually and this is a race condition with task cancelation? Otherwise, there may be a logic bug somewhere."
                    );
                    continue;
                }

                let mut active_torrent: TorrentActiveModel = torrent.into();
                active_torrent.torrent_file = Set(Some(torrent_file));
                let torrent = active_torrent
                    .update(&self.operator.state.database)
                    .await
                    .unwrap();

                // TODO: log errors instead of failing
                self.operator
                    .torrent()
                    .log_update(TorrentOperation::ResolveMagnet {
                        id: torrent.id,
                        name: torrent.name.to_string(),
                    })
                    .await
                    .unwrap();

                log::info!("Torrent file for magnet {finished_id} has been saved to DB");
            }
        }
    }
}
