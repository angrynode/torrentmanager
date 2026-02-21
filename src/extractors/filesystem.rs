use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use itertools::Itertools;
use snafu::prelude::*;

use std::str::FromStr;

use crate::database::category::{self, CategoryOperator};
use crate::database::content_folder::{self, ContentFolderOperator, PathBreadcrumb};
use crate::extractors::normalized_path::NormalizedPathRelative;
use crate::routes::filesystem::FileSystemEntry;
use crate::state::AppState;
use crate::state::error::*;

#[derive(Clone, Debug)]
pub struct FileSystemView {
    pub category: category::Model,
    pub folder: Option<content_folder::Model>,
    pub children: Vec<FileSystemEntry>,
    pub ancestors: Vec<PathBreadcrumb>,
}

impl FromRequestParts<AppState> for FileSystemView {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        app_state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        log::info!("TEST");
        let Path(s) = Path::<String>::from_request_parts(parts, app_state)
            .await
            .unwrap();

        // Remove leading/trailing slash (the latter is more likely) but
        // neither should happen with the links we produce.
        let path = s.trim_start_matches("/").trim_end_matches("/");
        let path = NormalizedPathRelative::from_str(path).context(FileSystemNormalizationSnafu)?;

        // TODO: NormalizedPath is not yet foolproof, see new test that allows
        // empty strings
        let mut components = path.components();
        let category_name = {
            let Some(category_name) = components.next() else {
                return Err(AppStateError::FileSystemPath { s: s.to_string() });
            };
            category_name.to_string()
        };

        let folder_path: String = components.join("/");
        log::info!("{folder_path}");

        let category_operator = CategoryOperator::new(app_state.clone(), None);
        let content_folder_operator = ContentFolderOperator::new(app_state.clone(), None);

        let category: category::Model = category_operator
            .find_by_name(category_name)
            .await
            .context(CategorySnafu)?;

        let (folder, children) = if folder_path.is_empty() {
            let category_children = category_operator
                .list_folders(category.id)
                .await
                .context(CategorySnafu)?;
            (
                None,
                category_children
                    .into_iter()
                    .map(|x| FileSystemEntry::from_content_folder(&category, &x))
                    .collect(),
            )
        } else {
            let content_folder = content_folder_operator
                // TODO: why do we have absolute paths in the DB???
                // must format to add "/" in front of path like in DB
                .find_by_path(format!("/{}", folder_path))
                .await
                .context(ContentFolderSnafu)?;

            let content_folder_children = content_folder_operator
                .list_child_folders(content_folder.id)
                .await
                .context(ContentFolderSnafu)?;

            (
                Some(content_folder),
                content_folder_children
                    .into_iter()
                    .map(|x| FileSystemEntry::from_content_folder(&category, &x))
                    .collect(),
            )
        };

        let ancestors = PathBreadcrumb::for_filesystem_path(&path);

        Ok(Self {
            category,
            folder,
            children,
            ancestors,
        })
    }
}
