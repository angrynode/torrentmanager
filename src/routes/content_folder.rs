use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use crate::database::content_folder::PathBreadcrumb;
use crate::database::{category, content_folder};
use crate::extractors::folder_request::FolderRequest;
use crate::filesystem::FileSystemEntry;
use crate::state::AppStateContext;
use crate::state::flash_message::{
    FallibleTemplate, FlashRedirect, FlashTemplate, OperationStatus, StatusCookie,
};

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
}

impl ContentFolderShowTemplate {
    fn new(context: AppStateContext, folder: FolderRequest) -> Self {
        let FolderRequest {
            breadcrumbs,
            category,
            children,
            folder,
        } = folder;

        Self {
            breadcrumbs,
            category,
            children,
            flash: None,
            folder,
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
    folder: FolderRequest,
    status: StatusCookie,
) -> FlashTemplate<ContentFolderShowTemplate> {
    status.with_template(ContentFolderShowTemplate::new(context, folder))
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
            Err(status.with_template(ContentFolderShowTemplate::new(context, folder)))
        }
    }
}
