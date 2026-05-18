use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::content_folder::PathBreadcrumb;
use crate::database::{category, content_folder, magnet, torrent};
use crate::extractors::folder_request::FolderRequest;
use crate::extractors::moving::MovingQuery;
use crate::filesystem::FileSystemEntry;
use crate::state::flash_message::{FallibleTemplate, FlashRedirect, OperationStatus, StatusCookie};
use crate::state::{AppStateContext, error::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContentFolderForm {
    pub name: String,
    pub parent_id: Option<i32>,
    pub category_id: i32,
}

#[derive(Template, WebTemplate)]
#[template(path = "content_folders/show.html")]
pub struct ContentFolderShowTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// current folder
    pub folder: content_folder::Model,
    /// Folders with parent_id set to current folder
    pub children: Vec<FileSystemEntry>,
    /// Category
    pub category: category::Model,
    /// BreadCrumb extract from path
    pub breadcrumbs: Vec<PathBreadcrumb>,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
    /// Related magnets in this folder
    pub magnets: Vec<magnet::Model>,
    /// Related torrents in this folder
    pub torrents: Vec<torrent::Model>,
    /// If any, the current torrent being moved in the folder.
    pub current_torrent: Option<torrent::Model>,
}

impl ContentFolderShowTemplate {
    fn new(
        context: AppStateContext,
        folder: FolderRequest,
        current_torrent: Option<torrent::Model>,
    ) -> Self {
        let FolderRequest {
            breadcrumbs,
            category,
            children,
            folder,
            magnets,
            torrents,
        } = folder;

        Self {
            breadcrumbs,
            category,
            children,
            flash: None,
            folder,
            state: context,
            magnets,
            torrents,
            current_torrent,
        }
    }
}

impl FallibleTemplate for ContentFolderShowTemplate {
    fn with_optional_flash(&mut self, flash: Option<OperationStatus>) {
        self.flash = flash;
    }
}

pub async fn show(
    context: AppStateContext,
    folder: FolderRequest,
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
                .update_category_content_folder(
                    torrent.clone(),
                    folder.category.clone(),
                    Some(folder.folder.clone()),
                )
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
                .with_template(ContentFolderShowTemplate::new(
                    context,
                    folder,
                    Some(torrent),
                ))
                .into_response())
        }
    } else {
        Ok(status
            .with_template(ContentFolderShowTemplate::new(context, folder, None))
            .into_response())
    }
}

pub async fn create_subfolder(
    context: AppStateContext,
    jar: CookieJar,
    folder: FolderRequest,
    Form(form): Form<ContentFolderForm>,
) -> Result<FlashRedirect, ContentFolderShowTemplate> {
    match context.db.content_folder().create(&form).await {
        Ok(created) => {
            let status = StatusCookie::success(
                jar,
                format!(
                    "The folder {} has been successfully created (ID: {})",
                    created.name, created.id
                ),
            );

            let uri = format!("/folders/{}{}", folder.category.name, created.path);
            Ok(status.redirect(&uri))
        }
        Err(error) => {
            let status = OperationStatus::error(error);
            Err(status.with_template(ContentFolderShowTemplate::new(context, folder, None)))
        }
    }
}
