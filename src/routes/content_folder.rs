use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::response::Redirect;
use axum_extra::extract::CookieJar;
use camino::Utf8PathBuf;
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
    pub path: String,
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

pub async fn create(
    context: AppStateContext,
    jar: CookieJar,
    Form(mut form): Form<ContentFolderForm>,
) -> Result<FlashRedirect, AppStateError> {
    let categories = context.db.category();
    let content_folders = context.db.content_folder();

    // build path with Parent folder path (or category path if parent is None) + folder.name
    let parent_path = if let Some(parent_id) = form.parent_id {
        let parent_folder = content_folders
            .find_by_id(parent_id)
            .await
            .context(ContentFolderSnafu)?;
        Utf8PathBuf::from(parent_folder.path)
    } else {
        Utf8PathBuf::new()
    };

    // Get folder category
    let category: category::Model = categories
        .find_by_id(form.category_id)
        .await
        .context(CategorySnafu)?;

    // If name contains "/" returns an error
    if form.name.contains("/") {
        let status = StatusCookie::error(
            jar,
            format!(
                "Failed to create Folder, {} is not valid (it contains '/')",
                form.name
            ),
        );

        let uri = format!("/folders/{}{}", category.name, parent_path.into_string());
        return Ok(status.redirect(&uri));
    }

    // build final path with parent_path and path of form
    form.path = format!("{}/{}", parent_path, form.name);

    let created = content_folders.create(&form).await;

    match created {
        Ok(created) => {
            tokio::fs::create_dir_all(format!("{}/{}", category.path, created.path.clone()))
                .await
                .context(IOSnafu)?;

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
        // TODO: why don't we produce an error here?
        Err(_error) => Ok((jar, Redirect::to("/"))),
    }
}
