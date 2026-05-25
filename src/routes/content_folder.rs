use askama::Template;
use askama_web::WebTemplate;
use axum::extract::Form;
use axum::extract::Path;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use crate::database::content_folder;
use crate::state::AppStateContext;
use crate::state::error::AppStateError;
use crate::state::flash_message::{
    FallibleTemplate, FlashRedirect, FlashTemplate, OperationStatus, StatusCookie,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContentFolderForm {
    pub name: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "content_folders/show.html")]
pub struct ContentFolderShowTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Current folder, unless we're on the index page
    pub folder: Option<content_folder::Model>,
    /// Folders with parent_id set to current folder
    // TODO: order by alphanumeric
    pub children: Vec<content_folder::Model>,
    /// Ancestors leading to this page (breadcrumb)
    pub ancestors: Vec<content_folder::Model>,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
}

impl ContentFolderShowTemplate {
    fn new(context: AppStateContext, folder: content_folder::FolderView) -> Self {
        let content_folder::FolderView {
            ancestors,
            children,
            folder,
        } = folder;

        Self {
            children,
            flash: None,
            folder,
            ancestors,
            state: context,
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
) -> Result<FlashTemplate<ContentFolderShowTemplate>, AppStateError> {
    // 404 if requested ID does not exist
    let view = content_folder::FolderView::from_id(&context.db.content_folder(), id).await?;
    Ok(status.with_template(ContentFolderShowTemplate::new(context, view)))
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

// TODO: create top-level
pub async fn create_subfolder(
    context: AppStateContext,
    jar: CookieJar,
    Path(id): Path<i32>,
    Form(form): Form<ContentFolderForm>,
) -> Result<Result<FlashRedirect, ContentFolderShowTemplate>, AppStateError> {
    let view = content_folder::FolderView::from_id(&context.db.content_folder(), id).await?;
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
