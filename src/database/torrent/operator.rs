use hightorrent_api::hightorrent::{MagnetLink, TorrentFile, TorrentID};
use sea_orm::*;
use snafu::prelude::*;

use std::ops::Deref;

use crate::database::content_folder;
use crate::database::operation::Table;
use crate::database::operator::{DatabaseOperator, TableOperator};
use crate::resolver::ResolverAction;

use super::*;

#[derive(Clone, Debug)]
pub struct TorrentOperator<'a> {
    pub db: &'a DatabaseOperator,
}

impl Deref for TorrentOperator<'_> {
    type Target = DatabaseOperator;

    fn deref(&self) -> &DatabaseOperator {
        self.db
    }
}

impl TableOperator for TorrentOperator<'_> {
    fn table(&self) -> Table {
        Table::Torrent
    }
}

impl TorrentOperator<'_> {
    /// List torrents with related content_folder
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_with_related(&self) -> Result<Vec<ModelEx>, TorrentError> {
        Entity::load()
            .with(content_folder::Entity)
            .all(&self.state.database)
            .await
            .context(DBSnafu)
    }

    /// List torrents
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list(&self) -> Result<Vec<Model>, TorrentError> {
        Entity::find()
            .all(&self.state.database)
            .await
            .context(DBSnafu)
    }

    /// List torrents for a given content folder
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_for_folder(
        &self,
        folder: &content_folder::Model,
    ) -> Result<Vec<Model>, TorrentError> {
        // TODO: optimization
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|x| x.content_folder_id == folder.id)
            .collect())
    }

    /// Count torrents
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn count(&self) -> Result<usize, TorrentError> {
        // TODO: there may be a faster sea_orm operation for this
        Ok(self.list().await?.len())
    }

    pub async fn get(&self, id: i32) -> Result<Model, TorrentError> {
        let db = &self.state.database;

        Entity::find_by_id(id)
            .one(db)
            .await
            .context(DBSnafu)?
            .ok_or(TorrentError::NotFound { id })
    }

    pub async fn get_by_torrent_id(&self, id: &TorrentID) -> Result<Model, TorrentError> {
        let db = &self.state.database;

        Entity::find()
            .filter(Column::TorrentId.eq(id.clone()))
            .one(db)
            .await
            .context(DBSnafu)?
            .ok_or(TorrentError::NotFoundTorrentID { id: id.clone() })
    }

    /// Import a magnet link
    ///
    /// Fails if:
    ///
    /// - the torrent is already imported
    /// - the magnet file is invalid
    pub async fn import_magnet(
        &self,
        folder: content_folder::Model,
        magnet: String,
    ) -> Result<Model, TorrentError> {
        let magnet = MagnetLink::new(&magnet).context(InvalidLinkSnafu)?;
        let torrent_id = magnet.id();

        // Check duplicates
        let list = self.list().await?;

        if list.iter().find(|x| x.torrent_id == torrent_id).is_some() {
            return Err(TorrentError::Duplicate { torrent_id });
        }

        let model = ActiveModel {
            torrent_id: Set(torrent_id.clone()),
            torrent_file: Set(None),
            magnet_link: Set(magnet.clone()),
            name: Set(magnet.name().to_string()),
            status: Set(TorrentStatus::Resolving),
            content_folder_id: Set(folder.id),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)?;

        // Should not fail
        let model = model.try_into_model().unwrap();

        self.log_create(TorrentOperation::ImportMagnet {
            id: model.id,
            name: magnet.name().to_string(),
            folder: (folder.id, folder.name.to_string()),
        })
        .await
        .context(LoggerSnafu)?;

        // Now that the magnet has been summoned into the DB,
        // we should let the resolver know about it.
        self.state
            .resolver
            .send(ResolverAction::Resolve(magnet))
            .expect("resolver sender channel has been closed");

        Ok(model)
    }

    /// Import a torrent file. If there was previously an unresolved magnet
    /// with the same torrent ID, the previous folder will be overriden.
    ///
    /// Fails if:
    ///
    /// - the torrent is already imported (and resolved if it was a magnet link)
    /// - the torrent file is invalid
    pub async fn import_torrent(
        &self,
        folder: content_folder::Model,
        file: axum::body::Bytes,
    ) -> Result<Model, TorrentError> {
        let torrent = TorrentFile::from_slice(&file).context(InvalidFileSnafu)?;
        let torrent_id = torrent.id();

        // Check duplicates
        let list = self.list().await?;

        if let Some(already_torrent) = list.iter().find(|x| x.torrent_id == torrent_id) {
            return self
                .update_from_torrent_file(already_torrent.clone(), folder, torrent)
                .await;
        }

        let model = ActiveModel {
            torrent_id: Set(torrent.id()),
            torrent_file: Set(Some(torrent.clone())),
            magnet_link: Set(torrent.magnet_link().unwrap()),
            name: Set(torrent.name().to_string()),
            status: Set(TorrentStatus::Downloading),
            content_folder_id: Set(folder.id),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)?;

        // Should not fail
        let model = model.try_into_model().unwrap();

        self.log_create(TorrentOperation::ImportTorrent {
            id: model.id,
            name: torrent.name,
            folder: (folder.id, folder.name.to_string()),
        })
        .await
        .context(LoggerSnafu)?;

        Ok(model)
    }

    /// Internal method used by `import_torrent`.
    ///
    /// Resolves a magnet to a torrent, or if it's already resolved, errors because of duplicate.
    /// Also updates the associated content folder.
    async fn update_from_torrent_file(
        &self,
        torrent: Model,
        folder: content_folder::Model,
        file: TorrentFile,
    ) -> Result<Model, TorrentError> {
        if torrent.torrent_file.is_some() {
            // TODO: merge trackers found in magnet/torrent?
            return Err(TorrentError::Duplicate {
                torrent_id: torrent.torrent_id,
            });
        }

        let previous_folder = self
            .content_folder()
            .find_by_id(torrent.content_folder_id)
            .await
            .unwrap();

        let mut active_model: ActiveModel = torrent.clone().into();
        active_model.torrent_file = Set(Some(file));
        active_model.status = Set(TorrentStatus::Downloading);

        let mut folder_operation_log = None;

        // Override previous folder if needed
        if previous_folder.id != folder.id {
            active_model.content_folder_id = Set(folder.id);
            folder_operation_log = Some(TorrentOperation::MoveTorrent {
                id: torrent.id,
                name: torrent.name.clone(),
                previous_folder: (previous_folder.id, previous_folder.name.to_string()),
                new_folder: (folder.id, folder.name.to_string()),
            });
        }

        let torrent = active_model
            .update(&self.state.database)
            .await
            .context(DBSnafu)?;

        // Cancel any pending resolution operation
        self.state
            .resolver
            .send(ResolverAction::Cancel(torrent.torrent_id.clone()))
            .expect("resolver sender channel has been closed");

        self.log_update(TorrentOperation::ResolveMagnet {
            id: torrent.id,
            name: torrent.name.to_string(),
        })
        .await
        .context(LoggerSnafu)?;

        if let Some(operation) = folder_operation_log {
            self.log_update(operation).await.context(LoggerSnafu)?;
        }

        Ok(torrent)
    }
}
