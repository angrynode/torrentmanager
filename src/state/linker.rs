use snafu::prelude::*;

use crate::database::operator::DatabaseOperator;
use crate::database::{category, content_folder};
use crate::state::AppState;
use crate::state::error::*;

pub struct Linker {
    pub state: AppState,
}

impl Linker {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn db(&self) -> DatabaseOperator {
        DatabaseOperator {
            state: self.state.clone(),
            user: None,
        }
    }

    /// Generate a link to a category / content-folder from typed models.
    ///
    /// We support any type that implements `Into<Model>` and Clone because
    /// i couldn't find a way to go from torrent::ModelEx which has loaded
    /// related ModelEx, to related Model.
    pub fn folder_url<T: Clone + Into<category::Model>, U: Clone + Into<content_folder::Model>>(
        &self,
        category: &T,
        content_folder: Option<&U>,
    ) -> String {
        let category: category::Model = category.clone().into();
        let content_folder: Option<content_folder::Model> =
            content_folder.map(|x| x.clone().into());

        // TODO: baseurl
        if let Some(content_folder) = content_folder {
            format!("/folders/{}{}", category.name, content_folder.path)
        } else {
            format!("/folders/{}", category.name)
        }
    }

    /// Generate a link to a category / content-folder
    /// from their IDs, which is a fallible operation.
    pub async fn try_folder_url(
        &self,
        category: i32,
        content_folder: Option<i32>,
    ) -> Result<String, AppStateError> {
        let category = self
            .db()
            .category()
            .find_by_id(category)
            .await
            .context(CategorySnafu)?;

        if let Some(content_folder) = content_folder {
            let content_folder = self
                .db()
                .content_folder()
                .find_by_id(content_folder)
                .await
                .context(ContentFolderSnafu)?;

            return Ok(self.folder_url(&category, Some(&content_folder)));
        }

        Ok(self.folder_url(&category, None::<&content_folder::Model>))
    }
}
