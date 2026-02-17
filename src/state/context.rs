use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::request::Parts;

use super::*;

/// Basic templating context used across pages.
///
/// Loading it may fail for some reasons, but it's so rare
/// and unrecoverable that it will trigger a global error
/// by rendering the AppStateError into an axum Response.
pub struct AppStateContext {
    pub errors: Vec<AppStateError>,
    pub free_space: FreeSpace,
    pub state: AppState,
    pub user: Option<User>,
}

impl FromRequestParts<AppState> for AppStateContext {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self {
            errors: vec![],
            free_space: state.free_space()?,
            state: state.clone(),
            user: User::from_request_parts(parts, state).await?,
        })
    }
}
