use axum::extract::{FromRequestParts, OptionalFromRequestParts};
use axum::http::request::Parts;

use crate::database::operator::DatabaseOperator;

use super::*;

/// Basic templating context used across pages.
///
/// Loading it may fail for some reasons, but it's so rare
/// and unrecoverable that it will trigger a global error
/// by rendering the AppStateError into an axum Response.
#[derive(Debug)]
pub struct AppStateContext {
    pub db: DatabaseOperator,
    // TODO: maybe use Arc<Vec<AppStateError>> to make it clonable? Or
    // Vec<Box<dyn Error>> ?
    pub errors: Vec<AppStateError>,
    pub free_space: FreeSpace,
    pub resolved_magnets_count: usize,
    pub state: AppState,
    pub user: Option<User>,
}

impl AppStateContext {
    /// Clones the overall context, dropping errors.
    ///
    /// AppStateError has many variants, not all of which can implement clone.
    pub fn partial_clone(&self) -> Self {
        // Destructure so we don't forget new fields in the future
        let Self {
            db,
            errors: _,
            free_space,
            resolved_magnets_count,
            state,
            user,
        } = self;

        Self {
            db: db.clone(),
            errors: vec![],
            free_space: free_space.clone(),
            resolved_magnets_count: *resolved_magnets_count,
            state: state.clone(),
            user: user.clone(),
        }
    }
}

impl FromRequestParts<AppState> for AppStateContext {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = User::from_request_parts(parts, state).await?;
        let db = DatabaseOperator::new(state.clone(), user.clone());
        let resolved_magnets_count = db
            .magnet()
            .list_resolved()
            .await
            .context(MagnetUploadSnafu)?
            .len();

        Ok(Self {
            db,
            errors: vec![],
            free_space: state.free_space()?,
            resolved_magnets_count,
            state: state.clone(),
            user,
        })
    }
}
