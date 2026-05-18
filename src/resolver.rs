use hightorrent_api::hightorrent::{MagnetLink, TorrentFile, TorrentID};
use librqbit::*;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

use std::collections::HashMap;
use std::sync::Arc;

use crate::database::magnet::MagnetOperator;
use crate::state::AppState;

/// A magnet link resolver
///
/// The resolver keeps track of background tasks (JoinHandle) but
/// currently has no mechanism to cancel them, i.e. to remove
/// a magnet being resolved.
///
/// In the future, we'd also like to have a push mechanism, so we
/// only read the magnet table once on startup, and then listen
/// for updates on a channel determined on startup. This will
/// avoid polluting the logs with queries to read the table.
pub struct Resolver {
    operator: MagnetOperator,
    // Channel to receive new magnets to resolve
    receiver: UnboundedReceiver<MagnetLink>,
    session: Arc<Session>,
    state: AppState,
    // Keep track of background tasks resolving torrent files from magnets
    // In the future, this will allow to cancel/delete tasks.
    tasks: HashMap<TorrentID, JoinHandle<TorrentFile>>,
}

impl Resolver {
    /// Initialize rqbit in the background
    pub async fn new(state: AppState, receiver: UnboundedReceiver<MagnetLink>) -> Self {
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
            operator: MagnetOperator {
                state: state.clone(),
                user: None,
            },
            receiver,
            session,
            state,
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
        log::info!("Starting the magnet resolver: checking for known magnets to resolve.");
        for magnet in self.operator.list().await.unwrap() {
            // Start a new resolving task
            self.resolve(magnet.link);
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
                    let magnet_link = recv.expect("magnetlink resolver sender has been closed");
                    self.resolve(magnet_link);
                }
            };
        }
    }

    /// Check if some tasks have finished resolving, and save result in the DB.
    pub async fn save_resolved(&mut self) {
        let finished_ids: Vec<TorrentID> = self
            .tasks
            .iter()
            .filter_map(|(k, v)| {
                if v.is_finished() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        for finished_id in finished_ids {
            log::info!("Magnet {finished_id} has finished resolving. Saving to DB.");

            // Get the raw task handle, removing it from the active tasks
            let handle = self.tasks.remove(&finished_id).unwrap();
            let torrent_file = handle.await.unwrap();

            // If the magnet was deleted for some reason, this is a no-op
            if let Ok(magnet) = self
                .operator
                .get_by_torrent_id(&finished_id)
                .await {
                if let Err(e) = self
                    .operator
                    .delete(magnet.id)
                    .await {
                    log::info!("Magnet {finished_id} failed to delete from magnet table. Maybe it was already deleted? {e}");
                    continue
                }

            }

            // TODO: save to the torrent DB
            log::info!("Torrent file for magnet {finished_id} has been saved to DB");
        }
    }
}
