use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;
use axum_extra::extract::CookieJar;
use snafu::prelude::*;

// TUTORIAL: https://github.com/SeaQL/sea-orm/blob/master/examples/axum_example/
use crate::database::category::{self, CategoryOperator};
use crate::state::flash_message::{OperationStatus, get_cookie};
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Categories
    pub categories: Vec<category::Model>,
    /// Operation status for UI confirmation
    pub flash: Option<OperationStatus>,
}

#[derive(Template, WebTemplate)]
#[template(path = "upload.html")]
pub struct UploadTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Categories
    pub categories: Vec<String>,
}

impl IndexTemplate {
    pub async fn new(
        app_state_context: AppStateContext,
        app_state: AppState,
        jar: CookieJar,
    ) -> Result<(CookieJar, Self), AppStateError> {
        let categories = CategoryOperator::new(app_state.clone(), app_state_context.user.clone())
            .list()
            .await
            .context(CategorySnafu)?;

        let (jar, operation_status) = get_cookie(jar);

        Ok((
            jar,
            IndexTemplate {
                state: app_state_context,
                categories,
                flash: operation_status,
            },
        ))
    }
}

impl UploadTemplate {
    pub async fn new(
        app_state_context: AppStateContext,
        app_state: AppState,
    ) -> Result<Self, AppStateError> {
        let categories: Vec<String> =
            CategoryOperator::new(app_state.clone(), app_state_context.user.clone())
                .list()
                .await
                .context(CategorySnafu)?
                .into_iter()
                .map(|x| x.name.to_string())
                .collect();

        Ok(UploadTemplate {
            state: app_state_context,
            categories,
        })
    }
}

pub async fn index(
    app_state_context: AppStateContext,
    State(app_state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, IndexTemplate), AppStateError> {
    IndexTemplate::new(app_state_context, app_state, jar).await
}

pub async fn upload(
    app_state_context: AppStateContext,
    State(app_state): State<AppState>,
) -> Result<UploadTemplate, AppStateError> {
    UploadTemplate::new(app_state_context, app_state).await
}
