use askama::Template;
use askama_web::WebTemplate;
use axum::extract::Form;
use axum::extract::Path;
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::database::{content_folder, torrent};
use crate::state::AppStateContext;
use crate::state::error::AppStateError;
use crate::state::flash_message::{
    FallibleTemplate, FlashRedirect, FlashTemplate, OperationStatus, StatusCookie,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContentFolderForm {
    pub name: String,
}

/// A request to move a torrent around in the categories/folders.
///
/// When validate is set, the requested folder is set to the database.
#[derive(Clone, Debug, Deserialize)]
#[serde_as]
pub struct MoveTorrentQuery {
    #[serde(default)]
    #[serde_as(as = "DeserializeFromStr")]
    pub moving_id: Option<i32>,
    #[serde(default)]
    pub moving_validate: bool,
}

#[derive(Template, WebTemplate)]
#[template(path = "content_folders/show.html")]
pub struct ContentFolderShowTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Current folder, unless we're on the index page
    pub folder: Option<content_folder::Model>,
    /// Torrents associated with the content folder (empty on index page)
    pub torrents: Vec<torrent::Model>,
    /// Folders with parent_id set to current folder
    // TODO: order by alphanumeric
    pub children: Vec<content_folder::Model>,
    /// Ancestors leading to this page (breadcrumb)
    pub ancestors: Vec<content_folder::Model>,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
    /// Torrent being moved (if any)
    pub moving_torrent: Option<torrent::Model>,
}

impl ContentFolderShowTemplate {
    pub fn new(context: AppStateContext, folder: content_folder::FolderView) -> Self {
        let content_folder::FolderView {
            ancestors,
            children,
            folder,
            torrents,
            moving_torrent,
        } = folder;

        Self {
            children,
            flash: None,
            folder,
            torrents,
            ancestors,
            state: context,
            moving_torrent,
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
    Path(id): Path<i32>,
    status: StatusCookie,
    Query(moving): Query<MoveTorrentQuery>,
) -> Result<Response, AppStateError> {
    // 404 if requested ID does not exist
    let view =
        content_folder::FolderView::from_id(&context.db.content_folder(), id, moving.moving_id)
            .await?;

    if view.moving_torrent.is_some() && moving.moving_validate {
        // Request to effectively move the torrent to this folder
        // Once done, redirect to the same page
        if let Err(e) = context
            .db
            .torrent()
            .move_folder(
                view.moving_torrent.clone().unwrap(),
                view.folder.as_ref().unwrap(),
            )
            .await
        {
            return Ok(OperationStatus::error(e)
                .with_template(ContentFolderShowTemplate::new(context, view))
                .into_response());
        }

        // Success! Perform a redirection
        return Ok(status
            .with_success("Torrent successfully moved".to_string())
            .redirect(&format!("/folders/{}", view.folder.as_ref().unwrap().id))
            .into_response());
    }

    Ok(status
        .with_template(ContentFolderShowTemplate::new(context, view))
        .into_response())
}

pub async fn index(
    context: AppStateContext,
    status: StatusCookie,
) -> Result<FlashTemplate<ContentFolderShowTemplate>, AppStateError> {
    let view = content_folder::FolderView::index(&context.db.content_folder()).await?;
    Ok(status.with_template(ContentFolderShowTemplate::new(context, view)))
}

pub async fn create_folder(
    context: AppStateContext,
    jar: CookieJar,
    Form(form): Form<ContentFolderForm>,
) -> Result<Result<FlashRedirect, ContentFolderShowTemplate>, AppStateError> {
    let view = content_folder::FolderView::index(&context.db.content_folder()).await?;
    match context
        .db
        .content_folder()
        .create(view.folder.clone(), form.name)
        .await
    {
        Ok(created) => {
            let status = StatusCookie::success(
                jar,
                format!(
                    "The folder {} has been successfully created (ID: {})",
                    created.name, created.id
                ),
            );

            let uri = format!("/folders/{}", created.id);
            Ok(Ok(status.redirect(&uri)))
        }
        Err(error) => {
            let status = OperationStatus::error(error);
            Ok(Err(status.with_template(ContentFolderShowTemplate::new(
                context, view,
            ))))
        }
    }
}

pub async fn create_subfolder(
    context: AppStateContext,
    jar: CookieJar,
    Path(id): Path<i32>,
    Form(form): Form<ContentFolderForm>,
) -> Result<Result<FlashRedirect, ContentFolderShowTemplate>, AppStateError> {
    let view = content_folder::FolderView::from_id(&context.db.content_folder(), id, None).await?;
    match context
        .db
        .content_folder()
        .create(view.folder.clone(), form.name)
        .await
    {
        Ok(created) => {
            let status = StatusCookie::success(
                jar,
                format!(
                    "The folder {} has been successfully created (ID: {})",
                    created.name, created.id
                ),
            );

            let uri = format!("/folders/{}", created.id);
            Ok(Ok(status.redirect(&uri)))
        }
        Err(error) => {
            let status = OperationStatus::error(error);
            Ok(Err(status.with_template(ContentFolderShowTemplate::new(
                context, view,
            ))))
        }
    }
}
