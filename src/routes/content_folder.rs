use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::category::CategoryOperator;
use crate::database::content_folder::ContentFolderOperator;
use crate::database::{category, content_folder};
use crate::extractors::user::User;
use crate::state::flash_message::{OperationStatus, get_cookie};
use crate::state::{AppState, AppStateContext, error::*};

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
    pub sub_content_folders: Vec<content_folder::Model>,
    /// Logged-in user.
    pub user: Option<User>,
    /// Category
    pub category: category::Model,
    /// BreadCrumb extract from path
    pub breadcrumb_items: Vec<PathBreadcrumb>,
    /// Parent Folder if exist. If None, the parent is category
    pub parent_folder: Option<content_folder::Model>,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
}

pub struct PathBreadcrumb {
    pub name: String,
    pub path: String,
}

pub async fn show(
    State(app_state): State<AppState>,
    user: Option<User>,
    Path((_category_name, folder_path)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<(CookieJar, ContentFolderShowTemplate), AppStateError> {
    let app_state_context = app_state.context().await?;

    let content_folder_operator = ContentFolderOperator::new(app_state.clone(), user.clone());

    // get current content folders with Path
    let current_content_folder = content_folder_operator
        // must format to add "/" in front of path like in DB
        .find_by_path(format!("/{}", folder_path))
        .await
        .context(ContentFolderSnafu)?;

    // Get all sub content folders of the current folder
    let sub_content_folders: Vec<content_folder::Model> = content_folder_operator
        .list_child_folders(current_content_folder.id)
        .await
        .context(ContentFolderSnafu)?;

    // Get current categories
    let category: category::Model = CategoryOperator::new(app_state.clone(), user.clone())
        .find_by_id(current_content_folder.category_id)
        .await
        .context(CategorySnafu)?;

    // create breadcrumb with ancestor of current folders
    let mut content_folder_ancestors: Vec<PathBreadcrumb> = Vec::new();
    // To get Current Parent Folder
    let mut parent_folder: Option<content_folder::Model> = None;

    content_folder_ancestors.push(PathBreadcrumb {
        name: current_content_folder.name.clone(),
        path: current_content_folder.path.clone(),
    });

    let mut current_id = current_content_folder.parent_id;
    while let Some(id) = current_id {
        let folder = content_folder_operator
            .find_by_id(id)
            .await
            .context(ContentFolderSnafu)?;

        if parent_folder.is_none() {
            parent_folder = Some(folder.clone());
        }

        content_folder_ancestors.push(PathBreadcrumb {
            name: folder.name,
            path: folder.path,
        });

        current_id = folder.parent_id;
    }

    // Reverse the ancestor to create Breadrumb
    content_folder_ancestors.reverse();

    let (jar, operation_status) = get_cookie(jar);

    Ok((
        jar,
        ContentFolderShowTemplate {
            parent_folder,
            breadcrumb_items: content_folder_ancestors,
            sub_content_folders,
            current_content_folder,
            category,
            state: app_state_context,
            user,
            flash: operation_status,
        },
    ))
}

pub async fn create(
    State(app_state): State<AppState>,
    user: Option<User>,
    jar: CookieJar,
    Form(mut form): Form<ContentFolderForm>,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    // let app_state_context = app_state.context().await?;
    let content_folder = ContentFolderOperator::new(app_state.clone(), user.clone());

    // build path with Parent folder path (or category path if parent is None) + folder.name
    let parent_path = if let Some(parent_id) = form.parent_id {
        let parent_folder = content_folder
            .find_by_id(parent_id)
            .await
            .context(ContentFolderSnafu)?;
        Utf8PathBuf::from(parent_folder.path)
    } else {
        Utf8PathBuf::new()
    };

    // Get folder category
    let category: category::Model = CategoryOperator::new(app_state.clone(), user.clone())
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

    let created = content_folder.create(&form, user.clone()).await;

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
