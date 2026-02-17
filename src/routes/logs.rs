use askama::Template;
use askama_web::WebTemplate;
use snafu::prelude::*;

use crate::database::operation::OperationLog;
use crate::database::operation::OperationType;
use crate::state::{AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "logs/index.html")]
pub struct LogTemplate {
    pub state: AppStateContext,
    pub logs: Vec<OperationLog>,
}

pub async fn index(context: AppStateContext) -> Result<LogTemplate, AppStateError> {
    let logs = context.state.logger.read().await.context(LoggerSnafu)?;

    Ok(LogTemplate {
        state: context,
        logs,
    })
}
