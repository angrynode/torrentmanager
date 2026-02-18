use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::content_folder::PathBreadcrumb;
use crate::database::{category, content_folder};
use crate::extractors::folder_request::FolderRequest;
use crate::filesystem::FileSystemEntry;
use crate::routes::magnet::MagnetForm;
use crate::state::flash_message::{OperationStatus, get_cookie};
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
    pub current_content_folder: content_folder::Model,
    /// Folders with parent_id set to current folder
    pub children: Vec<FileSystemEntry>,
    /// Category
    pub category: category::Model,
    /// BreadCrumb extract from path
    pub breadcrumbs: Vec<PathBreadcrumb>,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
    // TODO: WIP
    pub error: Option<AppStateError>,
}

impl ContentFolderShowTemplate {
    fn new(context: AppStateContext, folder: FolderRequest) -> Self {
        Self {
            breadcrumbs: folder.breadcrumbs,
            children: folder.children,
            current_content_folder: folder.folder,
            category: folder.category,
            state: context,
            flash: None,
            error: None,
        }
    }

    fn with_flash(mut self, flash: Option<OperationStatus>) -> Self {
        self.flash = flash;
        self
    }

    fn with_errored_form(mut self, _form: MagnetForm, error: AppStateError) -> Self {
        self.error = Some(error);
        self
    }
}

pub async fn show(
    context: AppStateContext,
    folder: FolderRequest,
    jar: CookieJar,
) -> Result<(CookieJar, ContentFolderShowTemplate), AppStateError> {
    let (jar, operation_status) = get_cookie(jar);

    Ok((
        jar,
        ContentFolderShowTemplate::new(context, folder).with_flash(operation_status),
    ))
}

pub async fn post_magnet(
    context: AppStateContext,
    folder: FolderRequest,
    Form(form): Form<MagnetForm>,
) -> Result<Redirect, ContentFolderShowTemplate> {
    // TODO: proper error type
    if let Err(e) = context
        .db
        .magnet()
        .create(&form)
        .await
        .context(MagnetUploadSnafu)
    {
        return Err(ContentFolderShowTemplate::new(context, folder).with_errored_form(form, e));
    }

    // TODO: what to do when upload is successful?
    Ok(Redirect::to("/magnet"))
}

pub async fn create(
    context: AppStateContext,
    jar: CookieJar,
    Form(mut form): Form<ContentFolderForm>,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
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
        let operation_status = OperationStatus {
            success: false,
            message: format!(
                "Failed to create Folder, {} is not valid (it contains '/')",
                form.name
            ),
        };
        let jar = operation_status.set_cookie(jar);

        let uri = format!("/folders/{}{}", category.name, parent_path.into_string());

        return Ok((jar, Redirect::to(uri.as_str()).into_response()));
    }

    // build final path with parent_path and path of form
    form.path = format!("{}/{}", parent_path, form.name);

    let created = content_folders.create(&form).await;

    match created {
        Ok(created) => {
            tokio::fs::create_dir_all(format!("{}/{}", category.path, created.path.clone()))
                .await
                .context(IOSnafu)?;

            let operation_status = OperationStatus {
                success: true,
                message: format!(
                    "The folder {} has been successfully created (ID: {})",
                    created.name, created.id
                ),
            };

            let jar = operation_status.set_cookie(jar);
            let uri = format!("/folders/{}{}", category.name, created.path);

            Ok((jar, Redirect::to(uri.as_str()).into_response()))
        }
        Err(_error) => Ok((jar, Redirect::to("/").into_response())),
    }
}
