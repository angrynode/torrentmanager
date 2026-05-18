use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::request::Parts;

use crate::database::operator::DatabaseOperator;
use crate::state::linker::Linker;

use super::*;

/// Basic templating context used across pages.
///
/// Loading it may fail for some reasons, but it's so rare
/// and unrecoverable that it will trigger a global error
/// by rendering the AppStateError into an axum Response.
pub struct AppStateContext {
    pub db: DatabaseOperator,
    pub free_space: FreeSpace,
    pub linker: Linker,
    pub state: AppState,
    pub user: Option<User>,
    pub magnets_count: usize,
}

impl FromRequestParts<AppState> for AppStateContext {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state).await?;
        let db = DatabaseOperator::new(state.clone(), user.clone());
        let magnets_count = db.magnet().count().await.context(MagnetUploadSnafu)?;

        Ok(Self {
            db: DatabaseOperator::new(state.clone(), user.clone()),
            free_space: state.free_space()?,
            linker: Linker::new(state.clone()),
            magnets_count,
            state: state.clone(),
            user,
        })
    }
}
