use camino::Utf8PathBuf;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use snafu::prelude::*;

use crate::extractors::user::User;
use crate::routes::category::CategoryForm;
use crate::state::AppState;
use crate::state::error::{self as state_error, AppStateError};

/// A category to store associated files.
///
/// Each category has a name and an associated path on disk, where
/// symlinks to the content will be created.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub path: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

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
    pub async fn list(&self) -> Result<Vec<Model>, AppStateError> {
        Entity::find()
            .all(&self.state.database)
            .await
            .context(state_error::SqliteSnafu)
    }

    /// Create a new category, creating the corresponding directory.
    ///
    /// Fails if:
    ///
    /// - name or path is already taken (they should be unique)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn create(&self, f: &CategoryForm) -> Result<Model, AppStateError> {
        let dir = Utf8PathBuf::from(&f.path);
        let parent = dir.parent().unwrap();

        if !tokio::fs::try_exists(parent)
            .await
            .context(IOSnafu)
            .context(state_error::CategorySnafu)?
        {
            return Err(CategoryError::ParentDir {
                path: parent.to_string(),
            })
            .context(state_error::CategorySnafu);
        }

        // Check duplicates
        let list = self.list().await?;
        if list.iter().any(|x| x.name == f.name) {
            return Err(CategoryError::NameTaken {
                name: f.name.clone(),
            })
            .context(state_error::CategorySnafu);
        }
        if list.iter().any(|x| x.path == f.path) {
            return Err(CategoryError::PathTaken {
                path: f.path.clone(),
            })
            .context(state_error::CategorySnafu);
        }

        let model = ActiveModel {
            name: Set(f.name.clone()),
            path: Set(f.path.clone()),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)
        .context(state_error::CategorySnafu)?;

        Ok(model.try_into_model().unwrap())
    }
}
