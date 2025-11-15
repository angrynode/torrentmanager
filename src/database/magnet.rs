use chrono::Utc;
use hightorrent_api::hightorrent::{MagnetLink, MagnetLinkError, TorrentID};
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use crate::database::operation::*;
use crate::database::{category, content_folder};
use crate::extractors::user::User;
use crate::routes::magnet::MagnetForm;
use crate::state::AppState;
use crate::state::logger::LoggerError;

/// A category to store associated files.
///
/// Each category has a name and an associated path on disk, where
/// symlinks to the content will be created.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "magnet")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub torrent_id: TorrentID,
    pub link: MagnetLink,
    pub name: String,
    pub resolved: bool,
    pub content_folder_id: i32,
    #[sea_orm(belongs_to, from = "content_folder_id", to = "id")]
    pub content_folder: HasOne<content_folder::Entity>,
    pub category_id: i32,
    #[sea_orm(belongs_to, from = "category_id", to = "id")]
    pub category: HasOne<category::Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum MagnetError {
    #[snafu(display("The magnet is invalid"))]
    InvalidMagnet { source: MagnetLinkError },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("The magnet (ID: {id}) does not exist"))]
    NotFound { id: i32 },
    #[snafu(display("The magnet (TorrentID: {id}) does not exist"))]
    NotFoundTorrentID { id: TorrentID },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
}

#[derive(Clone, Debug)]
pub struct MagnetOperator {
    pub state: AppState,
    pub user: Option<User>,
}

impl MagnetOperator {
    /// List magnets
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list(&self) -> Result<Vec<Model>, MagnetError> {
        Entity::find()
            .all(&self.state.database)
            .await
            .context(DBSnafu)
    }

    pub async fn get(&self, id: i32) -> Result<Model, MagnetError> {
        let db = &self.state.database;

        Entity::find_by_id(id)
            .one(db)
            .await
            .context(DBSnafu)?
            .ok_or(MagnetError::NotFound { id })
    }

    pub async fn get_by_torrent_id(&self, id: &TorrentID) -> Result<Model, MagnetError> {
        let db = &self.state.database;

        Entity::find()
            .filter(Column::TorrentId.eq(id.clone()))
            .one(db)
            .await
            .context(DBSnafu)?
            .ok_or(MagnetError::NotFoundTorrentID { id: id.clone() })
    }

    /// Delete an uploaded magnet
    pub async fn delete(&self, id: i32) -> Result<String, MagnetError> {
        let db = &self.state.database;

        let uploaded_magnet = Entity::find_by_id(id)
            .one(db)
            .await
            .context(DBSnafu)?
            .ok_or(MagnetError::NotFound { id })?;

        let clone: Model = uploaded_magnet.clone();
        uploaded_magnet.delete(db).await.context(DBSnafu)?;

        let operation_log = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            table: Table::Magnet,
            operation: OperationType::Delete,
            operation_id: OperationId {
                object_id: clone.id,
                name: clone.name.to_owned(),
            },
            operation_form: None,
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(clone.name)
    }

    /// Create a new uploaded magnet
    ///
    /// Fails if:
    ///
    /// - the magnet is invalid
    pub async fn create(&self, f: &MagnetForm) -> Result<Model, MagnetError> {
        let magnet = MagnetLink::new(&f.magnet).context(InvalidMagnetSnafu)?;

        // Check duplicates
        let list = self.list().await?;

        if list.iter().any(|x| x.torrent_id == magnet.id()) {
            // The magnet is already known
            return self.get_by_torrent_id(&magnet.id()).await;
        }

        let model = ActiveModel {
            torrent_id: Set(magnet.id()),
            link: Set(magnet.clone()),
            name: Set(magnet.name().to_string()),
            // TODO: check if we already have the torrent in which case it's already resolved!
            resolved: Set(false),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)?;

        // Should not fail
        let model = model.try_into_model().unwrap();

        let operation_log = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            table: Table::Magnet,
            operation: OperationType::Create,
            operation_id: OperationId {
                object_id: model.id.to_owned(),
                name: model.name.to_string(),
            },
            operation_form: Some(Operation::Magnet(f.clone())),
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }
}
