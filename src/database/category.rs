use camino::Utf8PathBuf;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use crate::database::operation::*;
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
    pub name: String,
    #[sea_orm(unique)]
    pub path: String,
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
    #[snafu(display("The category ({details}) does not exist"))]
    NotFound { details: String },
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
    pub async fn find_by_id(&self, id: i32) -> Result<Model, CategoryError> {
        let category = Entity::find_by_id(id)
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match category {
            Some(category) => Ok(category),
            None => Err(CategoryError::NotFound {
                details: format!("ID: {}", id),
            }),
        }
    }

    /// Find one category by Name
    pub async fn find_by_name(&self, name: String) -> Result<Model, CategoryError> {
        let category = Entity::find()
            .filter(Column::Name.contains(name.clone()))
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match category {
            Some(category) => Ok(category),
            None => Err(CategoryError::NotFound {
                details: format!("NAME: {}", name),
            }),
        }
    }

    /// Delete a category
    pub async fn delete(&self, id: i32, user: Option<User>) -> Result<String, CategoryError> {
        let db = &self.state.database;
        let category: Option<Model> = Entity::find_by_id(id).one(db).await.context(DBSnafu)?;

        match category {
            Some(category) => {
                let category_clone: Model = category.clone();
                category.delete(db).await.context(DBSnafu)?;

                let operation_log = OperationLog {
                    user,
                    date: Utc::now(),
                    table: Table::Category,
                    operation: OperationType::Delete,
                    operation_id: OperationId {
                        object_id: category_clone.id,
                        name: category_clone.name.to_owned(),
                    },
                    operation_form: None,
                };

                self.state
                    .logger
                    .write(operation_log)
                    .await
                    .context(LoggerSnafu)?;

                Ok(category_clone.name)
            }
            None => Err(CategoryError::NotFound {
                details: format!("ID: {}", id),
            }),
        }
    }

    /// Create a new category, creating the corresponding directory.
    ///
    /// Fails if:
    ///
    /// - name or path is already taken (they should be unique)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn create(
        &self,
        f: &CategoryForm,
        user: Option<User>,
    ) -> Result<Model, CategoryError> {
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
                name: f.name.clone(),
            });
        }
        if list.iter().any(|x| x.path == f.path) {
            return Err(CategoryError::PathTaken {
                path: f.path.clone(),
            });
        }

        // Normalized path to avoid trailing slash
        let normalized_path = dir.components().collect::<Utf8PathBuf>();
        let model = ActiveModel {
            name: Set(f.name.clone()),
            path: Set(normalized_path.into_string()),
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
