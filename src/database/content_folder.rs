use camino::Utf8PathBuf;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use std::str::FromStr;

use crate::database::category::{self, CategoryError};
use crate::database::magnet;
use crate::database::operation::{Operation, OperationId, OperationLog, OperationType, Table};
use crate::database::operator::DatabaseOperator;
use crate::database::torrent;
use crate::extractors::normalized_path::{NormalizedPathAbsolute, NormalizedPathComponent};
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
    // TODO: Here the path is absolute, even though it's technically
    // relative to the parent folder/category. This is because we want
    // to make it easy to look up by path.
    //
    // This means if we ever implement moving/renaming folders/categories,
    // we'll need to recursively change all dependent paths.
    //
    // Maybe we'd like to revisit this design decision at some point.
    pub path: NormalizedPathAbsolute,
    pub category_id: i32,
    #[sea_orm(belongs_to, from = "category_id", to = "id")]
    pub category: HasOne<category::Entity>,
    pub parent_id: Option<i32>,
    #[sea_orm(self_ref, relation_enum = "Parent", from = "parent_id", to = "id")]
    pub parent: HasOne<Entity>,
    #[sea_orm(has_many)]
    pub magnets: HasMany<magnet::Entity>,
    #[sea_orm(has_many)]
    pub torrents: HasMany<torrent::Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ContentFolderError {
    #[snafu(display("There is already a content folder called `{name}` in the current folder."))]
    NameTaken { name: String },
    #[snafu(display("The folder name is invalid. It must not contain slashes."))]
    NameInvalid,
    #[snafu(display("The folder path must appear absolute"))]
    PathInvalid,
    #[snafu(display("The Content Folder (Path: {path}) does not exist"))]
    NotFound { path: String },
    #[snafu(display("The content folder id is invalid: {id}"))]
    IDInvalid { id: String },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
    #[snafu(display("Category operation failed"))]
    Category { source: CategoryError },
    #[snafu(display("Failed to create the folder on disk"))]
    IO { source: std::io::Error },
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

    pub fn db(&self) -> DatabaseOperator {
        DatabaseOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }

    /// list All child folders for 1 folder
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list_child_folders(
        &self,
        content_folder_id: i32,
    ) -> Result<Vec<Model>, ContentFolderError> {
        Entity::find()
            .filter(Column::ParentId.eq(content_folder_id))
            .all(&self.state.database)
            .await
            .context(DBSnafu)
    }

    /// Find one Content Folder by path
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn find_by_path(&self, path: String) -> Result<Model, ContentFolderError> {
        let path = NormalizedPathAbsolute::from_str(&path)
            .map_err(|_e| ContentFolderError::PathInvalid)?;

        let content_folder = Entity::find_by_path(path.clone())
            .one(&self.state.database)
            .await
            .context(DBSnafu)?;

        match content_folder {
            Some(category) => Ok(category),
            None => Err(ContentFolderError::NotFound {
                path: path.to_string(),
            }),
        }
    }

    /// Find one content folder by ID
    ///
    /// Fails if:
    ///
    /// - the requested ID does not exist
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

    /// Find one content folder by stringy ID
    ///
    /// Fails if:
    ///
    /// - the requested ID does not exist
    /// - the requested ID could not be parsed into an i32
    pub async fn find_by_id_str(&self, id: &str) -> Result<Model, ContentFolderError> {
        let id: i32 = id
            .parse()
            .map_err(|_e| ContentFolderError::IDInvalid { id: id.to_string() })?;
        self.find_by_id(id).await
    }

    /// Create a new content folder
    ///
    /// Fails if:
    ///
    /// - name is already taken (they should be unique in one folder)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn create(&self, f: &ContentFolderForm) -> Result<Model, ContentFolderError> {
        let name = NormalizedPathComponent::from_str(&f.name)
            .map_err(|_e| ContentFolderError::NameInvalid)?;

        let category = self
            .db()
            .category()
            .find_by_id(f.category_id)
            .await
            .context(CategorySnafu)?;

        // Check duplicates in same category/folder
        {
            let siblings = if let Some(parent_id) = f.parent_id {
                self.list_child_folders(parent_id).await?
            } else {
                self.db()
                    .category()
                    .list_folders(f.category_id)
                    .await
                    .context(CategorySnafu)?
            };
            if siblings.iter().any(|x| x.name == f.name) {
                return Err(ContentFolderError::NameTaken {
                    name: f.name.clone(),
                });
            }
        }

        // This path is an absolute path, but relative to a category path
        let inner_path = if let Some(parent_id) = f.parent_id {
            let parent = self.find_by_id(parent_id).await?;
            NormalizedPathAbsolute::from_str(&format!("{}/{}", parent.path, name,)).unwrap()
        } else {
            NormalizedPathAbsolute::from_str(&format!("/{}", name)).unwrap()
        };

        let model = ActiveModel {
            name: Set(name.to_string()),
            path: Set(inner_path.clone()),
            category_id: Set(f.category_id),
            parent_id: Set(f.parent_id),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)?;

        let real_path =
            NormalizedPathAbsolute::from_str(&format!("{}{}", category.path, inner_path)).unwrap();

        // TODO: we may want to remove the entry from the DB when creating the folder
        // on disk fails.
        tokio::fs::create_dir_all(&real_path)
            .await
            .context(IOSnafu)?;

        // Should not fail
        let model = model.try_into_model().unwrap();

        let operation_log = OperationLog {
            user: self.user.clone(),
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

#[derive(Clone, Debug, PartialEq)]
pub struct PathBreadcrumb {
    pub name: String,
    pub path: String,
}

impl PathBreadcrumb {
    /// Produce a list of path ancestors, ordered by descending order (parent -> child).
    ///
    /// This includes the current category/folder.
    ///
    /// The path may contain leading/trailing slashes, but any sufficiently
    /// weirder path may produce unexpected results.
    pub fn for_filesystem_path(path: &str) -> Vec<PathBreadcrumb> {
        let path = path.trim_start_matches("/").trim_end_matches("/");
        let mut breadcrumbs = vec![];
        let mut path = Utf8PathBuf::from(path.to_string());

        log::info!("{path}");
        breadcrumbs.push(PathBreadcrumb {
            name: path.file_name().unwrap().to_string(),
            path: path.to_string(),
        });

        while path.pop() {
            if path.as_str().is_empty() {
                break;
            }

            log::info!("{:?}", path.file_name());
            breadcrumbs.push(PathBreadcrumb {
                name: path.file_name().unwrap().to_string(),
                path: path.to_string(),
            });
        }

        breadcrumbs.into_iter().rev().collect()
    }
}
