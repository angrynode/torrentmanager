use camino::Utf8PathBuf;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use crate::database::{content_folder, operation::*};
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
    pub content_folders: HasMany<super::content_folder::Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CategoryError {
    #[snafu(display("There is already a category called `{name}`"))]
    NameTaken { name: String },
    #[snafu(display("There is already a category in dir `{path}`"))]
    PathTaken { path: String },
    #[snafu(display("The parent directory does not exist: {path}"))]
    ParentDir { path: String },
    #[snafu(display("Other disk error"))]
    IO { source: std::io::Error },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("The category (ID: {id}) does not exist"))]
    IDNotFound { id: i32 },
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
    /// Should not fail, unless SQLite was corrupted for some reason.
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
    pub async fn create(&self, f: &CategoryForm) -> Result<Model, CategoryError> {
        let dir = Utf8PathBuf::from(&f.path);
        let parent = dir.parent().unwrap();

        if !tokio::fs::try_exists(parent).await.context(IOSnafu)? {
            return Err(CategoryError::ParentDir {
                path: parent.to_string(),
            });
        }

        // Check duplicates
        let list = self.list().await?;

        if list.iter().any(|x| x.name == f.name) {
            return Err(CategoryError::NameTaken {
                name: f.name.to_string(),
            });
        }
        if list.iter().any(|x| x.path == f.path) {
            return Err(CategoryError::PathTaken {
                path: f.path.to_string(),
            });
        }

        let model = ActiveModel {
            name: Set(f.name.clone()),
            path: Set(f.path.clone()),
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
                name: f.name.to_string(),
            },
            operation_form: Some(Operation::Category(f.clone())),
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }
}
