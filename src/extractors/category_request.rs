use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use snafu::prelude::*;

use crate::database::category::{self, CategoryOperator};
use crate::database::content_folder::PathBreadcrumb;
use crate::filesystem::FileSystemEntry;
use crate::state::{AppState, error::*};

#[derive(Clone, Debug)]
pub struct CategoryRequest {
    pub category: category::Model,
    pub breadcrumbs: Vec<PathBreadcrumb>,
    pub children: Vec<FileSystemEntry>,
}

impl FromRequestParts<AppState> for CategoryRequest {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        app_state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Path(category_name) =
            <Path<String> as FromRequestParts<AppState>>::from_request_parts(parts, app_state)
                .await
                .unwrap();

        // Read-only operators: no need to extract the current user
        let categories = CategoryOperator::new(app_state.clone(), None);

        let category = categories
            .find_by_name(category_name.to_string())
            .await
            .context(CategorySnafu)?;

        // get all content folders in this category
        let content_folders = categories
            .list_folders(category.id)
            .await
            .context(CategorySnafu)?;

        let children = FileSystemEntry::from_content_folders(&category, &content_folders);

        let breadcrumbs = PathBreadcrumb::for_filesystem_path(category.name.as_str());

        Ok(Self {
            category,
            children,
            breadcrumbs,
        })
    }
}
