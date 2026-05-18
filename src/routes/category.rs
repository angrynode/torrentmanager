use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::category;
use crate::database::content_folder::PathBreadcrumb;
use crate::database::magnet;
use crate::database::torrent;
use crate::extractors::category_request::{CategoriesRequest, CategoryRequest};
use crate::extractors::moving::MovingQuery;
use crate::filesystem::FileSystemEntry;
use crate::routes::content_folder::ContentFolderForm;
use crate::routes::index::IndexTemplate;
use crate::state::flash_message::{FallibleTemplate, FlashRedirect, OperationStatus, StatusCookie};
use crate::state::{AppStateContext, error::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryForm {
    pub name: String,
    pub path: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/new.html")]
pub struct NewCategoryTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Default form with value
    pub category_form: Option<CategoryForm>,
}

pub async fn new(app_state_context: AppStateContext) -> NewCategoryTemplate {
    NewCategoryTemplate {
        state: app_state_context,
        category_form: None,
    }
}

pub async fn delete(
    context: AppStateContext,
    Path(id): Path<i32>,
    jar: CookieJar,
) -> FlashRedirect {
    let status = match context.db.category().delete(id).await {
        Ok(name) => StatusCookie::success(
            jar,
            format!("The category {} has been successfully deleted", name),
        ),
        Err(error) => StatusCookie::error(jar, error.to_string()),
    };

    status.redirect("/categories")
}

pub async fn create(
    context: AppStateContext,
    categories: CategoriesRequest,
    jar: CookieJar,
    Form(form): Form<CategoryForm>,
) -> Result<FlashRedirect, IndexTemplate> {
    match context.db.category().create(&form).await {
        Ok(created) => {
            let status = StatusCookie::success(
                jar,
                format!(
                    "The category {} has been successfully created (ID {})",
                    created.name, created.id
                ),
            );
            let uri = format!("/folders/{}", created.name);
            Ok(status.redirect(&uri))
        }
        Err(error) => {
            let status = OperationStatus::error(error);
            Err(status.with_template(IndexTemplate::new(context, categories)))
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/show.html")]
pub struct CategoryShowTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Categories found in database
    pub children: Vec<FileSystemEntry>,
    /// Category
    category: category::Model,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
    /// Breadcrumbs navigation
    pub breadcrumbs: Vec<PathBreadcrumb>,
    /// Magnets in this category
    pub magnets: Vec<magnet::Model>,
    /// Torrents in this category
    pub torrents: Vec<torrent::Model>,
    /// If any, the current torrent being moved in the folder.
    pub current_torrent: Option<torrent::Model>,
}

impl CategoryShowTemplate {
    fn new(
        context: AppStateContext,
        category: CategoryRequest,
        current_torrent: Option<torrent::Model>,
    ) -> Self {
        let CategoryRequest {
            breadcrumbs,
            category,
            children,
            magnets,
            torrents,
        } = category;

        Self {
            breadcrumbs,
            category,
            children,
            flash: None,
            state: context,
            magnets,
            torrents,
            current_torrent,
        }
    }
}

impl FallibleTemplate for CategoryShowTemplate {
    fn with_optional_flash(&mut self, flash: Option<OperationStatus>) {
        self.flash = flash;
    }
}

pub async fn show(
    context: AppStateContext,
    category: CategoryRequest,
    status: StatusCookie,
    Query(moving): Query<MovingQuery>,
) -> Result<Response, AppStateError> {
    if let Some(id) = moving.id {
        // We are currently moving a torrent between folders
        let torrent: torrent::Model = context
            .db
            .torrent()
            .get(id)
            .await
            .context(TorrentUploadSnafu)?;
        if moving.validate {
            // Save to DB the new location of the torrent
            let _torrent = context
                .db
                .torrent()
                .update_category_content_folder(torrent.clone(), category.category.clone(), None)
                .await
                .context(TorrentUploadSnafu)?;
            // Now we produce a redirection (to the same page) in order to refresh the list of torrents
            // in this folder, which was already computed.
            Ok(status
                .with_success("Torrent successfully saved to this folder".to_string())
                // Need to remove the query params
                .redirect("?")
                .into_response())
        } else {
            Ok(status
                .with_template(CategoryShowTemplate::new(context, category, Some(torrent)))
                .into_response())
        }
    } else {
        Ok(status
            .with_template(CategoryShowTemplate::new(context, category, None))
            .into_response())
    }
}

pub async fn create_folder(
    context: AppStateContext,
    jar: CookieJar,
    category: CategoryRequest,
    Form(form): Form<ContentFolderForm>,
) -> Result<FlashRedirect, CategoryShowTemplate> {
    match context.db.content_folder().create(&form).await {
        Ok(created) => {
            let status = StatusCookie::success(
                jar,
                format!(
                    "The folder {} has been successfully created (ID: {})",
                    created.name, created.id
                ),
            );

            let uri = format!("/folders/{}{}", category.category.name, created.path);
            Ok(status.redirect(&uri))
        }
        Err(error) => {
            let status = OperationStatus::error(error);
            Err(status.with_template(CategoryShowTemplate::new(context, category, None)))
        }
    }
}
