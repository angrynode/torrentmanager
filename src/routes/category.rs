use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::State;
// use sea_orm::entity::*;
use serde::Deserialize;

use crate::database::{category, category::CategoryOperator};
use crate::extractors::user::User;
use crate::state::{AppState, AppStateContext, error::*};

#[derive(Clone, Debug, Deserialize)]
pub struct CategoryForm {
    pub name: String,
    pub path: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "category.html")]
pub struct CategoryTemplate {
    /// Global application state
    pub state: AppStateContext,
    /// Category that was just created, to confirm in the UI
    pub created: Option<category::Model>,
    /// Categories found in database
    pub categories: Vec<category::Model>,
    /// Logged-in user.
    pub user: Option<User>,
}

pub async fn create(
    State(app_state): State<AppState>,
    user: Option<User>,
    Form(form): Form<CategoryForm>,
    // ) -> Result<CategoryTemplate, AppStateError> {
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    let app_state_context = app_state.context().await?;
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    let created = categories.create(&form).await?;
    Ok(CategoryTemplate {
        categories: categories.list().await?,
        created: Some(created),
        state: app_state_context,
        user,
    })
}

pub async fn index(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<CategoryTemplate, AppStateError> {
    let app_state_context = app_state.context().await?;
    let categories = CategoryOperator::new(app_state.clone(), user.clone());

    Ok(CategoryTemplate {
        categories: categories.list().await?,
        created: None,
        state: app_state_context,
        user,
    })
}
