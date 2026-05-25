use camino::{Utf8PathBuf, Utf8Path};
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use std::str::FromStr;

use crate::database::operation::{Operation, OperationLog, OperationType, Table};
use crate::database::operator::DatabaseOperator;
use crate::extractors::normalized_path::NormalizedPathComponent;
use crate::extractors::user::User;
use crate::state::AppState;
use crate::state::error::AppStateError;

/// A loaded folder, with all surrounding entities loaded as well:
///
/// - parents/ancestors, from the topmost to the closest
/// - direct children (non-recursive)
///
/// On the index page, `folder` is not populated.
#[derive(Clone, Debug)]
pub struct FolderView {
    pub ancestors: Vec<Model>,
    pub children: Vec<Model>,
    pub folder: Option<Model>,
}

impl FolderView {
    /// Loads the top-most folder view, which is not a folder and may not have parents.
    pub async fn index(operator: &ContentFolderOperator) -> Result<Self, ContentFolderError> {
        let children = operator.list().await?.into_iter().filter(|x| x.parent_id.is_none()).collect();
        
        Ok(Self {
            ancestors: vec!(),
            folder: None,
            children,
        })
    }
    
    /// Loads a folder view for a requested folder, if it exists.
    ///
    /// Fails when:
    ///
    /// - the requested ID does not exist
    // TODO: optimize with custom query
    pub async fn from_id(operator: &ContentFolderOperator, id: i32) -> Result<Self, ContentFolderError> {
        let list = operator.list().await?;

        if let Some(folder) = list.iter().find(|x| x.id == id) {
            Ok(Self {
                ancestors: folder.ancestors_from_list(&list).into_iter().cloned().collect(),
                children: folder.children_from_list(&list).into_iter().cloned().collect(),
                folder: Some(folder.clone()),
            })
        } else {
            Err(ContentFolderError::NotFound { id })
        }
    }
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
    
    /// List content folders
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn list(&self) -> Result<Vec<Model>, ContentFolderError> {
        Entity::find()
            .all(&self.state.database)
            .await
            .context(DBSnafu)
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
            None => Err(ContentFolderError::NotFound { id }),
        }
    }

    /// Create a new content folder
    ///
    /// Fails if:
    ///
    /// - name is already taken (they should be unique in one folder)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn create(&self, parent: Option<Model>, name: String) -> Result<Model, ContentFolderError> {
        let name = NormalizedPathComponent::from_str(&name)
            .map_err(|_e| ContentFolderError::NameInvalid)?;

        let list = self.list().await?;

        let siblings = parent.as_ref().map(|x| x.children_from_list(&list)).unwrap_or(vec!());
        if siblings.iter().any(|x| x.name == name) {
            return Err(ContentFolderError::NameTaken {
                name: name.to_string(),
            });
        }

        let model = ActiveModel {
            name: Set(name),
            parent_id: Set(parent.as_ref().map(|x| x.id)),
            ..Default::default()
        }
        .save(&self.state.database)
        .await
        .context(DBSnafu)?;

        // Should not fail
        let model = model.try_into_model().unwrap();

        // If the folder already exists, it's not an error. Maybe it was
        // created manually before importing to TorrentManager.
        let real_path = model.path_from_list(&self.state.config.media_dir, &list);
        if !tokio::fs::try_exists(&real_path).await.context(IOSnafu)? {
            tokio::fs::create_dir_all(&real_path).await.context(IOSnafu)?;
        }

        let operation_log = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            operation: ContentFolderOperation::Create {id: model.id, name: model.name.to_string(), parent: parent.as_ref().map(|x| (x.id, x.name.to_string())) }.into(),
            operation_type: OperationType::Create,
            table: Table::ContentFolder,
        };

        self.state
            .logger
            .write(operation_log)
            .await
            .context(LoggerSnafu)?;

        Ok(model)
    }
}
