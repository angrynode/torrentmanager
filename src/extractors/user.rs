use axum::{
    extract::OptionalFromRequestParts,
    http::{StatusCode, request::Parts},
};
use derive_more::Display;
use serde::{Deserialize, Serialize};

/// A logged-in user, as expressed by the Remote-User header.
///
/// Cannot be produced outside of header extraction.
#[derive(Clone, Debug, Display, Deserialize, Serialize)]
#[serde(transparent)]
pub struct User(pub String);

impl<S> OptionalFromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        if let Some(username) = parts.headers.get("remote-user") {
            match username.to_str() {
                Ok(username) => Ok(Some(User(String::from(username)))),
                Err(_e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "The remote-user header returned by the reverse proxy is invalid.",
                )),
            }
        } else {
            Ok(None)
        }
    }
}
