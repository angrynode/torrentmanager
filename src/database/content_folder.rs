use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use crate::database::category;
use crate::database::operation::{Operation, OperationId, OperationLog, OperationType, Table};
use crate::extractors::user::User;
use crate::routes::content_folder::ContentFolderForm;
use crate::state::AppState;
use crate::state::logger::LoggerError;

/// A content folder to store associated files.
///
/// Each content folder has a name and an associated path on disk, a Category
/// and it can have an Parent Content Folder (None if it's the first folder
/// in category)
#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq, Eq)]
#[sea_orm(table_name = "content_folder")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    #[sea_orm(unique)]
    pub path: String,
    pub category_id: i32,
    #[sea_orm(belongs_to, from = "category_id", to = "id")]
    pub category: HasOne<category::Entity>,
    pub parent_id: Option<i32>,
    #[sea_orm(self_ref, relation_enum = "Parent", from = "parent_id", to = "id")]
    pub parent: HasOne<Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ContentFolderError {
    #[snafu(display("There is already a content folder called `{name}`"))]
    NameTaken { name: String },
    #[snafu(display("There is already a content folder in dir `{path}`"))]
    PathTaken { path: String },
    #[snafu(display("The Content Folder (Path: {path}) does not exist"))]
    NotFound { path: String },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
}

#[derive(Clone, Debug)]
pub struct ContentFolderOperator {
    pub state: AppState,
    pub user: Option<User>,
}

impl ContentFolderOperator {
    pub fn new(state: AppState, user: Option<User>) -> Self {
        Self { state, user }
    }

    /// List Content Folders
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_by_parent_and_category(
        &self,
        parent_id: Option<i32>,
        category_id: i32,
    ) -> Result<Vec<Model>, ContentFolderError> {
        let mut query = Entity::find().filter(Column::CategoryId.eq(category_id));
        // parent_id can be None when it's the first folder in categories
        match parent_id {
            Some(parent_id) => query = query.filter(Column::ParentId.eq(parent_id)),
            None => query = query.filter(Column::ParentId.is_null()),
        }

        query.all(&self.state.database).await.context(DBSnafu)
    }

    /// Find one Content Folder by path
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn find_by_path(&self, path: String) -> Result<Model, ContentFolderError> {
        let content_folder = Entity::find_by_path(path.clone())
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match content_folder {
            Some(category) => Ok(category),
            None => Err(ContentFolderError::NotFound { path }),
        }
    }

    /// Find one Content Folder by ID
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn find_by_id(&self, id: i32) -> Result<Model, ContentFolderError> {
        let content_folder = Entity::find_by_id(id)
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match content_folder {
            Some(category) => Ok(category),
            None => Err(ContentFolderError::NotFound {
                path: id.to_string(),
            }),
        }
    }

    /// Create a new content folder
    ///
    /// Fails if:
    ///
    /// - name or path is already taken (they should be unique in one folder)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn create(
        &self,
        f: &ContentFolderForm,
        user: Option<User>,
    ) -> Result<Model, ContentFolderError> {
        // Check duplicates in same folder
        let list = self
            .list_by_parent_and_category(f.parent_id, f.category_id)
            .await?;

        if list.iter().any(|x| x.name == f.name) {
            return Err(ContentFolderError::NameTaken {
                name: f.name.clone(),
            });
        }

        if list.iter().any(|x| x.path == f.path) {
            return Err(ContentFolderError::PathTaken {
                path: f.path.clone(),
            });
        }

        let model = ActiveModel {
            name: Set(f.name.clone()),
            path: Set(f.path.clone()),
            category_id: Set(f.category_id),
            parent_id: Set(f.parent_id),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)?;

        // Should not fail
        let model = model.try_into_model().unwrap();

        let operation_log = OperationLog {
            user,
            date: Utc::now(),
            table: Table::ContentFolder,
            operation: OperationType::Create,
            operation_id: OperationId {
                object_id: model.id.to_owned(),
                name: f.name.to_string(),
            },
            operation_form: Some(Operation::ContentFolder(f.clone())),
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }
}
