use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

// TUTORIAL: https://github.com/SeaQL/sea-orm/blob/master/examples/axum_example/
use crate::extractors::user::User;
use crate::state::{AppState, AppStateContext, error::*};

use std::collections::HashMap;

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// TODO: Submitted values in a POST request
    ///
    /// This happens when the request was rejected by the handler, but still
    /// wants to repopulate form data from submitted values.
    pub post: HashMap<String, String>,
    /// Logged-in user.
    pub user: Option<User>,
    /// Categories
    pub categories: Vec<String>,
}

pub async fn index(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<Response, AppStateError> {
    let app_state_context = app_state.context().await?;

    let categories: Vec<String> = app_state
        .category_list()
        .await?
        .into_iter()
        .map(|x| x.name)
        .collect();
    if categories.is_empty() {
        Ok(crate::routes::category::index(State(app_state), user)
            .await?
            .into_response())
    } else {
        Ok(IndexTemplate {
            state: app_state_context,
            post: HashMap::new(),
            user,
            categories,
        }
        .into_response())
    }
}
