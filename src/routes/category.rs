use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::category::CategoryError;
use crate::database::{category, category::CategoryOperator};
use crate::extractors::user::User;
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CategoryForm {
    pub name: String,
    pub path: String,
}

pub struct OperationStatus {
    /// Status of operation
    pub success: bool,
    /// Message for confirmation alert
    pub message: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "categories/index.html")]
pub struct CategoriesTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Categories found in database
    pub categories: Vec<category::Model>,
    /// Logged-in user.
    pub user: Option<User>,
    /// Operation status for UI confirmation
    pub operation_status: Option<OperationStatus>,
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
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    let app_state_context = app_state.context().await?;
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    let deleted = categories.delete(id).await;

    match deleted {
        Ok(name) => Ok(CategoriesTemplate {
            categories: categories.list().await.context(CategorySnafu)?,
            operation_status: Some(OperationStatus {
                success: true,
                message: format!("The category {} has been successfully deleted", name),
            }),
            state: app_state_context,
            user,
        }),
        Err(error) => Ok(CategoriesTemplate {
            categories: categories.list().await.context(CategorySnafu)?,
            operation_status: Some(OperationStatus {
                success: false,
                message: format!("{}", error),
            }),
            state: app_state_context,
            user,
        }),
    }
}

pub async fn create(
    State(app_state): State<AppState>,
    user: Option<User>,
    Form(form): Form<CategoryForm>,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    let app_state_context = app_state.context().await?;
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    let created = categories.create(&form).await;

    match created {
        Ok(created) => Ok(CategoriesTemplate {
            categories: categories.list().await.context(CategorySnafu)?,
            state: app_state_context,
            user,
            operation_status: Some(OperationStatus {
                success: true,
                message: format!(
                    "The category {} has been successfully created (ID {})",
                    created.name, created.id
                ),
            }),
        }
        .into_response()),
        Err(error) => Ok(NewCategoryTemplate {
            state: app_state_context,
            user,
            category_form: Some(form),
            error: Some(error),
        }
        .into_response()),
    }
}

pub async fn index(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<CategoriesTemplate, AppStateError> {
    let app_state_context = app_state.context().await?;
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    Ok(CategoriesTemplate {
        categories: categories.list().await.context(CategorySnafu)?,
        state: app_state_context,
        user,
        operation_status: None,
    })
}
