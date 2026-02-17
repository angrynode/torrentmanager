use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use snafu::prelude::*;

use crate::database::category::{self, CategoryOperator};
use crate::database::content_folder::{self, ContentFolderOperator};
use crate::routes::content_folder::PathBreadcrumb;
use crate::state::AppState;
use crate::state::error::*;

#[derive(Clone, Debug)]
pub struct FolderRequest {
    pub category: category::Model,
    pub folder: content_folder::Model,
    pub sub_folders: Vec<content_folder::Model>,
    pub ancestors: Vec<PathBreadcrumb>,
    pub parent: Option<content_folder::Model>,
}

impl FromRequestParts<AppState> for FolderRequest {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        app_state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // This unwrap will only a category name is set, but no further folder path
        // However, that case is handled by a different route (`routes::category::show`).
        let Path((_category_name, folder_path)) = <Path<(String, String)> as FromRequestParts<
            AppState,
        >>::from_request_parts(parts, app_state)
        .await
        .unwrap();

        // Read-only operators: no need to extract the current user
        let category_operator = CategoryOperator::new(app_state.clone(), None);
        let content_folder_operator = ContentFolderOperator::new(app_state.clone(), None);

        // get current content folders with Path
        let current_content_folder = content_folder_operator
            // must format to add "/" in front of path like in DB
            .find_by_path(format!("/{}", folder_path))
            .await
            .context(ContentFolderSnafu)?;

        // Get all sub content folders of the current folder
        let sub_content_folders: Vec<content_folder::Model> = content_folder_operator
            .list_child_folders(current_content_folder.id)
            .await
            .context(ContentFolderSnafu)?;

        // Read-only operator
        let category: category::Model = category_operator
            .find_by_id(current_content_folder.category_id)
            .await
            .context(CategorySnafu)?;

        // create breadcrumb with ancestor of current folders
        let mut content_folder_ancestors: Vec<PathBreadcrumb> = Vec::new();
        // To get Current Parent Folder
        let mut parent_folder: Option<content_folder::Model> = None;

        content_folder_ancestors.push(PathBreadcrumb {
            name: current_content_folder.name.clone(),
            path: current_content_folder.path.clone(),
        });

        let mut current_id = current_content_folder.parent_id;
        while let Some(id) = current_id {
            let folder = content_folder_operator
                .find_by_id(id)
                .await
                .context(ContentFolderSnafu)?;

            if parent_folder.is_none() {
                parent_folder = Some(folder.clone());
            }

            content_folder_ancestors.push(PathBreadcrumb {
                name: folder.name,
                path: folder.path,
            });

            current_id = folder.parent_id;
        }

        // Reverse the ancestor to create Breadrumb
        content_folder_ancestors.reverse();

        Ok(Self {
            category,
            folder: current_content_folder,
            sub_folders: sub_content_folders,
            ancestors: content_folder_ancestors,
            parent: parent_folder,
        })
    }
}
