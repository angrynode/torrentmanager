use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::category::CategoryError;
use crate::database::content_folder;
use crate::database::{category, category::CategoryOperator};
use crate::extractors::normalized_path::*;
use crate::extractors::user::User;
use crate::state::flash_message::{OperationStatus, get_cookie};
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryForm {
    pub name: NormalizedPathComponent,
    pub path: NormalizedPathAbsolute,
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/new.html")]
pub struct NewCategoryTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Logged-in user.
    pub user: Option<User>,
    /// Error
    pub error: Option<CategoryError>,
    /// Default form with value
    pub category_form: Option<CategoryForm>,
}

pub async fn new(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    let app_state_context = app_state.context().await?;

    Ok(NewCategoryTemplate {
        state: app_state_context,
        user,
        category_form: None,
        error: None,
    })
}

pub async fn delete(
    State(app_state): State<AppState>,
    user: Option<User>,
    Path(id): Path<i32>,
    jar: CookieJar,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    // let app_state_context = app_state.context().await?;
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    let deleted = categories.delete(id).await;

    let operation_status = match deleted {
        Ok(name) => OperationStatus {
            success: true,
            message: format!("The category {} has been successfully deleted", name),
        },
        Err(error) => OperationStatus {
            success: false,
            message: format!("{}", error),
        },
    };

    let jar = operation_status.set_cookie(jar);

    Ok((jar, Redirect::to("/categories")))
}

pub async fn create(
    State(app_state): State<AppState>,
    user: Option<User>,
    jar: CookieJar,
    Form(form): Form<CategoryForm>,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    let created = categories.create(&form).await;

    match created {
        Ok(created) => {
            let operation_status = OperationStatus {
                success: true,
                message: format!(
                    "The category {} has been successfully created (ID {})",
                    created.name, created.id
                ),
            };

            let jar = operation_status.set_cookie(jar);

            Ok((jar, Redirect::to("/").into_response()))
        }
        Err(error) => {
            let operation_status = OperationStatus {
                success: false,
                message: format!("{}", error),
            };

            let jar = operation_status.set_cookie(jar);

            Ok((jar, Redirect::to("/").into_response()))
        }
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/show.html")]
pub struct CategoryShowTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Categories found in database
    pub content_folders: Vec<content_folder::Model>,
    /// Logged-in user.
    pub user: Option<User>,
    /// Category
    category: category::Model,
    /// Operation status for UI confirmation (Cookie)
    pub flash: Option<OperationStatus>,
}

pub async fn show(
    State(app_state): State<AppState>,
    user: Option<User>,
    Path(category_name): Path<String>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppStateError> {
    let app_state_context = app_state.context().await?;

    let category: category::Model = CategoryOperator::new(app_state.clone(), user.clone())
        .find_by_name(category_name.to_string())
        .await
        .context(CategorySnafu)?;

    // get all content folders in this category
    let content_folders: Vec<content_folder::Model> =
        CategoryOperator::new(app_state.clone(), user.clone())
            .list_folders(category.id)
            .await
            .context(CategorySnafu)?;

    let (jar, operation_status) = get_cookie(jar);

    Ok((
        jar,
        CategoryShowTemplate {
            content_folders,
            category,
            state: app_state_context,
            user,
            flash: operation_status,
        },
    ))
}
