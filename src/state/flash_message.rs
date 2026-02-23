use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::Redirect;
use axum_extra::extract::{CookieJar, cookie::Cookie};

use std::boxed::Box;

use crate::state::{AppState, AppStateError};

pub type FlashRedirect = (CookieJar, Redirect);
pub type FlashTemplate<T> = (CookieJar, T);

#[derive(Debug)]
pub enum MessageOrError {
    Message(String),
    Error(Box<dyn snafu::Error>),
}

impl MessageOrError {
    pub fn messages(&self) -> Vec<String> {
        match self {
            Self::Message(s) => vec![s.to_string()],
            Self::Error(e) => {
                let mut messages = vec![];
                messages.push(e.to_string());
                let mut error = e.source();
                while let Some(source) = error {
                    messages.push(source.to_string());
                    error = source.source();
                }
                messages
            }
        }
    }
}

/// A template which has an optional [`OperationStatus`].
pub trait FallibleTemplate: Sized {
    fn with_optional_flash(&mut self, flash: Option<OperationStatus>);

    fn add_error(&mut self, error: impl snafu::Error + Sized + 'static) {
        self.with_optional_flash(Some(OperationStatus {
            success: false,
            message: MessageOrError::Error(Box::new(error)),
        }))
    }
}

/// An operation success or failure, as seen in a template.
#[derive(Debug)]
pub struct OperationStatus {
    /// Status of operation
    pub success: bool,
    /// Message for confirmation alert
    pub message: MessageOrError,
}

impl OperationStatus {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message: MessageOrError::Message(message),
        }
    }

    pub fn error(error: impl snafu::Error + Sized + 'static) -> Self {
        Self {
            success: false,
            message: MessageOrError::Error(Box::new(error)),
        }
    }

    pub fn error_message(message: String) -> Self {
        Self {
            success: false,
            message: MessageOrError::Message(message),
        }
    }

    pub fn with_template<T: FallibleTemplate>(self, mut template: T) -> T {
        template.with_optional_flash(Some(self));
        template
    }
}

/// An operation status passed as a cookie.
///
/// to be passed between pages as a cookie (flash message).
///
/// On a route which may reads a flash message:
///
/// - use `StatusCookie` as an extractor, which will extract the status
///   from the cookie jar
/// - use [`StatusCookie::with_template`] to display the error and pass
///   along the emptied cookie jar
///
/// On a route which sets a flash message:
///
/// - use `CookieJar` as an extractor
/// - use [`StatusCookie::error`] or [`StatusCookie::success`] to add the cookie
/// - use [`StatusCookie::redirect`] to redirect with the extra cookie
#[derive(Clone, Debug)]
pub struct StatusCookie {
    pub cookies: CookieJar,
    pub message: Option<StatusMessage>,
}

impl StatusCookie {
    fn remove_cookies(&mut self) {
        let mut success = Cookie::new("operation_status_success", "".to_string());
        success.set_path("/");

        let mut message = Cookie::new("operation_status_message", "".to_string());
        message.set_path("/");

        self.cookies = self.cookies.clone().remove(success).remove(message);
    }

    fn add_cookies(&mut self, success: bool, message: String) {
        let mut success = Cookie::new("operation_status_success", success.to_string());
        success.set_path("/");

        let mut message = Cookie::new("operation_status_message", message);
        message.set_path("/");

        self.cookies = self.cookies.clone().add(success).add(message);
    }

    pub fn error(cookies: CookieJar, s: String) -> Self {
        let mut status = Self {
            cookies,
            message: Some(StatusMessage {
                success: false,
                message: s.clone(),
            }),
        };
        status.add_cookies(false, s);
        status
    }

    pub fn success(cookies: CookieJar, s: String) -> Self {
        let mut status = Self {
            cookies,
            message: Some(StatusMessage {
                success: true,
                message: s.clone(),
            }),
        };
        status.add_cookies(true, s);
        status
    }

    pub fn redirect(self, url: &str) -> FlashRedirect {
        (self.cookies, Redirect::to(url))
    }

    pub fn with_template<T: FallibleTemplate>(self, mut template: T) -> (CookieJar, T) {
        let cookies = self.cookies.clone();
        template.with_optional_flash(self.message.map(|m| m.into()));
        (cookies, template)
    }
}

impl From<StatusMessage> for OperationStatus {
    fn from(s: StatusMessage) -> Self {
        Self {
            success: s.success,
            message: MessageOrError::Message(s.message),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StatusMessage {
    success: bool,
    message: String,
}

impl FromRequestParts<AppState> for StatusCookie {
    type Rejection = AppStateError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state).await.unwrap();

        let status_message = match (
            jar.get("operation_status_success"),
            jar.get("operation_status_message"),
        ) {
            (Some(success), Some(message)) => Some(StatusMessage {
                success: if let Ok(success) = success.value().parse() {
                    success
                } else {
                    return Ok(StatusCookie {
                        cookies: jar,
                        message: None,
                    });
                },
                message: message.value().to_string(),
            }),
            _ => None,
        };

        let mut status = StatusCookie {
            cookies: jar,
            message: status_message,
        };
        status.remove_cookies();
        Ok(status)
    }
}
