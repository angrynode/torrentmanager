use askama::Template;
use askama_web::WebTemplate;
use axum::Form;
use axum::extract::State;
// use sea_orm::entity::*;
use sea_orm::*;
use serde::Deserialize;
use snafu::prelude::*;

use crate::database::{category, category::Entity as Category};
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
    _user: Option<User>,
    form: Form<CategoryForm>,
    // ) -> Result<CategoryTemplate, AppStateError> {
) -> Result<impl axum::response::IntoResponse, AppStateError> {
    println!("New category {:?}", form);
    let Form(CategoryForm { name, path }) = form;
    app_state.category_create(name, path).await?;
    Ok("foo")
}

pub async fn index(
    State(app_state): State<AppState>,
    user: Option<User>,
) -> Result<CategoryTemplate, AppStateError> {
    let app_state_context = app_state.context().await?;

    let categories = Category::find()
        .all(&app_state.database)
        .await
        .context(SqliteSnafu)?;

    Ok(CategoryTemplate {
        categories,
        created: None,
        state: app_state_context,
        user,
    })
}
