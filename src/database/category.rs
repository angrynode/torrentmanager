use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use std::str::FromStr;

use crate::database::operator::DatabaseOperator;
use crate::database::{content_folder, operation::*, torrent};
use crate::extractors::normalized_path::*;
use crate::extractors::user::User;
use crate::routes::category::CategoryForm;
use crate::state::AppState;
use crate::state::logger::LoggerError;

/// A category to store associated files.
///
/// Each category has a name and an associated path on disk, where
/// symlinks to the content will be created.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: NormalizedPathComponent,
    #[sea_orm(unique)]
    pub path: NormalizedPathAbsolute,
    #[sea_orm(has_many)]
    pub content_folders: HasMany<content_folder::Entity>,
    #[sea_orm(has_many)]
    pub torrents: HasMany<torrent::Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CategoryError {
    #[snafu(display("There is already a category called `{name}`"))]
    NameTaken { name: String },
    #[snafu(display("The category name is invalid. It must not contain slashes."))]
    NameInvalid,
    #[snafu(display("There is already a category in dir `{path}`"))]
    PathTaken { path: String },
    #[snafu(display("The category path is invalid. It must be an absolute path."))]
    PathInvalid,
    #[snafu(display("The parent directory does not exist: {path}"))]
    ParentDir { path: String },
    #[snafu(display("Other disk error"))]
    IO { source: std::io::Error },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("The category (ID: {id}) does not exist"))]
    IDNotFound { id: i32 },
    #[snafu(display("The category id is invalid: {id}"))]
    IDInvalid { id: String },
    #[snafu(display("The category (Name: {name}) does not exist"))]
    NameNotFound { name: String },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
}

#[derive(Clone, Debug)]
pub struct CategoryOperator {
    pub state: AppState,
    pub user: Option<User>,
}

impl CategoryOperator {
    pub fn new(state: AppState, user: Option<User>) -> Self {
        Self { state, user }
    }

    pub fn db(&self) -> DatabaseOperator {
        DatabaseOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }

    /// List categories
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list(&self) -> Result<Vec<Model>, CategoryError> {
        Entity::find()
            .all(&self.state.database)
            .await
            .context(DBSnafu)
    }

    /// Find one category by ID
    ///
    /// Fails if:
    ///
    /// - the requested ID does not exist
    pub async fn find_by_id(&self, id: i32) -> Result<Model, CategoryError> {
        let category = Entity::find_by_id(id)
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match category {
            Some(category) => Ok(category),
            None => Err(CategoryError::IDNotFound { id }),
        }
    }

    /// Find one category by stringy ID
    ///
    /// Fails if:
    ///
    /// - the requested ID does not exist
    /// - the requested ID could not be parsed into an i32
    pub async fn find_by_id_str(&self, id: &str) -> Result<Model, CategoryError> {
        let id: i32 = id
            .parse()
            .map_err(|_e| CategoryError::IDInvalid { id: id.to_string() })?;
        self.find_by_id(id).await
    }

    /// Find one category by Name
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn find_by_name(&self, name: String) -> Result<Model, CategoryError> {
        let category = Entity::find()
            .filter(Column::Name.contains(name.clone()))
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match category {
            Some(category) => Ok(category),
            None => Err(CategoryError::NameNotFound { name }),
        }
    }

    /// List folders for 1 category
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_folders(&self, id: i32) -> Result<Vec<content_folder::Model>, CategoryError> {
        let category = Entity::find_by_id(id)
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match category {
            Some(category) => {
                let folders = category
                    .find_related(content_folder::Entity)
                    // We only want the top-level folders
                    .filter(content_folder::Column::ParentId.is_null())
                    .all(&self.state.database)
                    .await
                    .context(DBSnafu)?;
                Ok(folders)
            }
            None => Err(CategoryError::IDNotFound { id }),
        }
    }

    /// Delete a category
    pub async fn delete(&self, id: i32) -> Result<String, CategoryError> {
        let db = &self.state.database;
        let category: Option<Model> = Entity::find_by_id(id).one(db).await.context(DBSnafu)?;

        match category {
            Some(category) => {
                let category_clone: Model = category.clone();
                category.delete(db).await.context(DBSnafu)?;

                let operation_log = OperationLog {
                    user: self.user.clone(),
                    date: Utc::now(),
                    table: Table::Category,
                    operation: OperationType::Delete,
                    operation_id: OperationId {
                        object_id: category_clone.id,
                        name: category_clone.name.to_string(),
                    },
                    operation_form: None,
                };

                self.state
                    .logger
                    .write(operation_log)
                    .await
                    .context(LoggerSnafu)?;

                Ok(category_clone.name.to_string())
            }
            None => Err(CategoryError::IDNotFound { id }),
        }
    }

    /// Create a new category, creating the corresponding directory.
    ///
    /// Fails if:
    ///
    /// - name or path is already taken (they should be unique)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn create(&self, form: &CategoryForm) -> Result<Model, CategoryError> {
        let name = NormalizedPathComponent::from_str(&form.name)
            .map_err(|_e| CategoryError::NameInvalid)?;
        let path = NormalizedPathAbsolute::from_str(&form.path)
            .map_err(|_e| CategoryError::PathInvalid)?;

        let dir = path.to_path_buf();
        let parent = dir.parent().unwrap();

        if !tokio::fs::try_exists(parent).await.context(IOSnafu)? {
            return Err(CategoryError::ParentDir {
                path: parent.to_string(),
            });
        }

        // Check duplicates
        let list = self.list().await?;

        if list.iter().any(|x| x.name == name) {
            return Err(CategoryError::NameTaken {
                name: name.to_string(),
            });
        }
        if list.iter().any(|x| x.path == path) {
            return Err(CategoryError::PathTaken {
                path: path.to_string(),
            });
        }

        let model = ActiveModel {
            name: Set(name.clone()),
            path: Set(path.clone()),
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
            table: Table::Category,
            operation: OperationType::Create,
            operation_id: OperationId {
                object_id: model.id.to_owned(),
                name: name.to_string(),
            },
            operation_form: Some(Operation::Category(form.clone())),
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }
}
