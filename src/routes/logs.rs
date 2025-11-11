use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;
use snafu::prelude::*;

use crate::database::operation::OperationLog;
use crate::extractors::user::User;
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "logs.html")]
pub struct LogTemplate {
    pub state: AppStateContext,
    pub logs: Vec<OperationLog>,
    pub user: Option<User>,
}

pub async fn index(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<LogTemplate, AppStateError> {
    let app_state_context = app_state.context().await?;
    let logs = app_state.logger.read().await.context(LoggerSnafu)?;

    Ok(LogTemplate {
        state: app_state_context,
        logs,
        user,
    })
}
