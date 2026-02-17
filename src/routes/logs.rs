use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;
use snafu::prelude::*;

use crate::database::operation::OperationLog;
use crate::database::operation::OperationType;
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "logs/index.html")]
pub struct LogTemplate {
    pub state: AppStateContext,
    pub logs: Vec<OperationLog>,
}

pub async fn index(
    app_state_context: AppStateContext,
    State(app_state): State<AppState>,
) -> Result<LogTemplate, AppStateError> {
    let logs = app_state.logger.read().await.context(LoggerSnafu)?;

    Ok(LogTemplate {
        state: app_state_context,
        logs,
    })
}
