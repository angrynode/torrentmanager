use chrono::Utc;
use hightorrent_api::hightorrent::{TorrentFile, TorrentFileError, TorrentID};
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use crate::database::operation::*;
use crate::database::operator::DatabaseOperator;
use crate::database::{category, content_folder};
use crate::extractors::user::User;
use crate::routes::torrent::TorrentForm;
use crate::state::AppState;
use crate::state::logger::LoggerError;

/// A category to store associated files.
///
/// Each category has a name and an associated path on disk, where
/// symlinks to the content will be created.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "torrent")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub torrent_id: TorrentID,
    pub file: TorrentFile,
    pub name: String,
    pub category_id: i32,
    #[sea_orm(belongs_to, from = "category_id", to = "id")]
    pub category: HasOne<category::Entity>,
    pub content_folder_id: Option<i32>,
    #[sea_orm(belongs_to, from = "content_folder_id", to = "id")]
    pub content_folder: HasOne<content_folder::Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum TorrentError {
    #[snafu(display("The torrent is invalid"))]
    InvalidFile { source: TorrentFileError },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("The torrent (ID: {id}) does not exist"))]
    NotFound { id: i32 },
    #[snafu(display("The torrent (TorrentID: {id}) does not exist"))]
    NotFoundTorrentID { id: TorrentID },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
    #[snafu(display("Requested category not found"))]
    NoSuchCategory { id: i32 },
    #[snafu(display("Requested content folder not found"))]
    NoSuchContentFolder { id: i32 },
}

#[derive(Clone, Debug)]
pub struct TorrentOperator {
    pub state: AppState,
    pub user: Option<User>,
}

impl TorrentOperator {
    pub fn new(state: AppState, user: Option<User>) -> Self {
        Self { state, user }
    }

    pub fn db(&self) -> DatabaseOperator {
        DatabaseOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }

    /// List torrents with related category/content_folder
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_with_related(&self) -> Result<Vec<ModelEx>, TorrentError> {
        // Entity::find()
        // method not found in `SelectTwoMany<database::torrent::Entity, database::category::Entity>`
        // .find_with_related(category::Entity)
        // .find_with_related(content_folder::Entity)
        //  the trait `sea_orm::EntityTrait` is not implemented for `(database::category::Entity, database::content_folder::Entity)
        // .find_with_related((category::Entity, content_folder::Entity))
        Entity::load()
            .with(category::Entity)
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

    /// List torrents in a specific category (without recursing)
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_for_category(&self, category_id: i32) -> Result<Vec<Model>, TorrentError> {
        // TODO: optimization
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|x| x.category_id == category_id && x.content_folder_id.is_none())
            .collect())
    }

    /// List torrents in a specific content_folder (without recursing)
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_for_content_folder(
        &self,
        content_folder_id: i32,
    ) -> Result<Vec<Model>, TorrentError> {
        // TODO: optimization
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|x| x.content_folder_id == Some(content_folder_id))
            .collect())
    }

    /// Count magnets
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

    /// Delete an uploaded magnet
    pub async fn delete(&self, id: i32) -> Result<String, TorrentError> {
        let db = &self.state.database;

        let uploaded_torrent = Entity::find_by_id(id)
            .one(db)
            .await
            .context(DBSnafu)?
            .ok_or(TorrentError::NotFound { id })?;

        let clone: Model = uploaded_torrent.clone();
        uploaded_torrent.delete(db).await.context(DBSnafu)?;

        let operation_log = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            table: Table::Torrent,
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
    /// - the torrent file is invalid
    pub async fn create(&self, f: &TorrentForm) -> Result<Model, TorrentError> {
        let torrent = TorrentFile::from_slice(&f.file).context(InvalidFileSnafu)?;

        // Check duplicates
        let list = self.list().await?;

        if list.iter().any(|x| x.torrent_id == torrent.id()) {
            // The torrent is already known
            return self.get_by_torrent_id(&torrent.id()).await;
        }

        // Verify that the requested category/content_folder exist
        let _category = self
            .db()
            .category()
            .find_by_id(f.category_id)
            .await
            .map_err(|_e| TorrentError::NoSuchCategory { id: f.category_id })?;

        if let Some(content_folder_id) = f.content_folder_id {
            let _content_folder = self
                .db()
                .content_folder()
                .find_by_id(content_folder_id)
                .await
                .map_err(|_e| TorrentError::NoSuchContentFolder {
                    id: content_folder_id,
                })?;
        }

        let model = ActiveModel {
            torrent_id: Set(torrent.id()),
            file: Set(torrent.clone()),
            name: Set(torrent.name().to_string()),
            category_id: Set(f.category_id),
            content_folder_id: Set(f.content_folder_id),
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
            table: Table::Torrent,
            operation: OperationType::Create,
            operation_id: OperationId {
                object_id: model.id.to_owned(),
                name: model.name.to_string(),
            },
            operation_form: Some(Operation::Torrent(f.clone())),
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }

    pub async fn update_category_content_folder(
        &self,
        model: Model,
        category: category::Model,
        content_folder: Option<content_folder::Model>,
    ) -> Result<Model, TorrentError> {
        let mut active: ActiveModel = model.clone().into();
        active.category_id = Set(category.id);
        active.content_folder_id = Set(content_folder.as_ref().map(|x| x.id));
        active.save(&self.state.database).await.context(DBSnafu)?;

        let operation_log = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            table: Table::Torrent,
            operation: OperationType::Update,
            operation_id: OperationId {
                object_id: model.id.to_owned(),
                name: model.name.to_string(),
            },
            operation_form: Some(Operation::MoveTorrent {
                torrent: model.id,
                category: category.id,
                content_folder: content_folder.map(|x| x.id),
            }),
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }
}
