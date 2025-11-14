use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;
use snafu::prelude::*;

// TUTORIAL: https://github.com/SeaQL/sea-orm/blob/master/examples/axum_example/
use crate::database::category::CategoryOperator;
use crate::extractors::user::User;
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Logged-in user.
    pub user: Option<User>,
    /// Categories
    pub categories: Vec<String>,
}

impl IndexTemplate {
    pub async fn new(app_state: AppState, user: Option<User>) -> Result<Self, AppStateError> {
        let categories: Vec<String> = CategoryOperator::new(app_state.clone(), user.clone())
            .list()
            .await
            .context(CategorySnafu)?
            .into_iter()
            .map(|x| x.name)
            .collect();

        Ok(IndexTemplate {
            state: app_state.context().await?,
            user,
            categories,
        })
    }
}

pub async fn index(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<IndexTemplate, AppStateError> {
    IndexTemplate::new(app_state, user).await
}
