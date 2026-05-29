use chrono::Utc;
use sea_orm::*;
use snafu::prelude::*;

use std::ops::Deref;
use std::str::FromStr;

use crate::database::operation::OperationLog;
use crate::database::operation::OperationType;
use crate::database::operation::Table;
use crate::database::operator::DatabaseOperator;
use crate::extractors::normalized_path::NormalizedPathComponent;

use super::*;

#[derive(Clone, Debug)]
pub struct ContentFolderOperator<'a> {
    pub db: &'a DatabaseOperator,
}

impl Deref for ContentFolderOperator<'_> {
    type Target = DatabaseOperator;

    fn deref(&self) -> &DatabaseOperator {
        self.db
    }
}

impl ContentFolderOperator<'_> {
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
    pub async fn create(
        &self,
        parent: Option<Model>,
        name: String,
    ) -> Result<Model, ContentFolderError> {
        let name = NormalizedPathComponent::from_str(&name)
            .map_err(|_e| ContentFolderError::NameInvalid)?;

        let list = self.list().await?;

        let siblings = parent
            .as_ref()
            .map(|x| x.children_from_list(&list))
            .unwrap_or(vec![]);
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
            tokio::fs::create_dir_all(&real_path)
                .await
                .context(IOSnafu)?;
        }

        let operation_log = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            operation: ContentFolderOperation::Create {
                id: model.id,
                name: model.name.to_string(),
                parent: parent.as_ref().map(|x| (x.id, x.name.to_string())),
            }
            .into(),
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
