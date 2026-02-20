use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::Path;
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};

use crate::database::category::CategoryError;
use crate::extractors::normalized_path::*;
use crate::state::flash_message::OperationStatus;
use crate::state::{AppStateContext, error::*};

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
    /// Error
    pub error: Option<CategoryError>,
    /// Default form with value
    pub category_form: Option<CategoryForm>,
}

pub async fn new(
    app_state_context: AppStateContext,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    Ok(NewCategoryTemplate {
        state: app_state_context,
        category_form: None,
        error: None,
    })
}

pub async fn delete(
    context: AppStateContext,
    Path(id): Path<i32>,
    jar: CookieJar,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    let operation_status = match context.db.category().delete(id).await {
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
    context: AppStateContext,
    jar: CookieJar,
    Form(form): Form<CategoryForm>,
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    match context.db.category().create(&form).await {
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
