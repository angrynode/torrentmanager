use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::content_folder::PathBreadcrumb;
use crate::database::{category, content_folder};
use crate::extractors::folder_request::FolderRequest;
use crate::filesystem::FileSystemEntry;
use crate::state::flash_message::{
    FallibleTemplate, FlashRedirect, FlashTemplate, OperationStatus, StatusCookie,
};
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

// TODO: currently an error takes us back to /
pub async fn create(
    context: AppStateContext,
    jar: CookieJar,
    Form(form): Form<ContentFolderForm>,
) -> Result<FlashRedirect, AppStateError> {
    let category = context
        .db
        .category()
        .find_by_id(form.category_id)
        .await
        .context(CategorySnafu)?;

    match context.db.content_folder().create(&form).await {
        Ok(created) => {
            let status = StatusCookie::success(
                jar,
                format!(
                    "The folder {} has been successfully created (ID: {})",
                    created.name, created.id
                ),
            );

            let uri = format!("/folders/{}{}", category.name, created.path);
            Ok(status.redirect(&uri))
        }
        Err(error) => {
            let status = StatusCookie::error(jar, error.to_string());
            let uri = if let Some(parent_id) = form.parent_id {
                let parent = context
                    .db
                    .content_folder()
                    .find_by_id(parent_id)
                    .await
                    .context(ContentFolderSnafu)?;
                format!("/folders/{}{}", category.name, parent.path)
            } else {
                format!("/folders{}", category.name)
            };
            Ok(status.redirect(&uri))
        }
    }
}
