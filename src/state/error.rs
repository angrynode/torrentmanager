use askama::Template;
use askama_web::WebTemplate;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use snafu::ErrorCompat;
use snafu::prelude::*;

use crate::database::category::CategoryError;
use crate::database::content_folder::ContentFolderError;
use crate::extractors::normalized_path::NormalizeError;
use crate::migration::DbErr as MigrationError;
use crate::state::free_space::FreeSpaceError;
use crate::state::logger::LoggerError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AppStateError {
    #[snafu(display("Failed to initialize the torrent API"))]
    InitAPI { source: hightorrent_api::ApiError },
    #[snafu(display("Failed to communicate with the torrent client"))]
    API { source: hightorrent_api::ApiError },
    #[snafu(display("Failed to get free space information"))]
    FreeSpace { source: FreeSpaceError },
    #[snafu(display("SQLite error"))]
    Sqlite { source: sea_orm::error::DbErr },
    #[snafu(display("Logger error"))]
    Logger { source: LoggerError },
    #[snafu(display("An other error occurred"))]
    Other {
        source: Box<dyn snafu::Error + Send + Sync + 'static>,
    },
    #[snafu(display("Category error"))]
    Category { source: CategoryError },
    #[snafu(display("Error during migration"))]
    Migration { source: MigrationError },
    #[snafu(display("Content folder error"))]
    ContentFolder { source: ContentFolderError },
    #[snafu(display("IO error"))]
    IO { source: std::io::Error },
    #[snafu(display("{reason}"))]
    Static { reason: &'static str },

    #[snafu(display("Invalid filesystem path normalization"))]
    FileSystemNormalization { source: NormalizeError },
    #[snafu(display("Invalid filesystem path"))]
    FileSystemPath { s: String },
}

impl AppStateError {
    pub fn inner_errors(&self) -> Vec<Box<dyn std::error::Error + '_>> {
        let mut inner_errors = vec![];
        for error in self.iter_chain().skip(1) {
            inner_errors.push(Box::new(error).into());
        }
        inner_errors
    }

    // Format an error for the logs
    pub fn log(&self) {
        log::error!("{self}");
        for error in self.iter_chain().skip(1) {
            log::error!("-> {error}");
        }
    }
}

/// Global error page generated from an [AppStateError].
#[derive(Debug, Template, WebTemplate)]
#[template(path = "error.html")]
pub struct AppStateErrorContext {
    state: AppStateErrorContextInner,
}

/// Helper struct so we can reuse base.html
/// with all it's `state.foo` expressions.
#[derive(Debug)]
pub struct AppStateErrorContextInner {
    // TODO: askama doesn't handle recursion well, so we convert
    // all errors to strings. Maybe related to:
    // https://github.com/askama-rs/askama/issues/393
    errors: Vec<AppStateError>,
}

impl From<AppStateError> for AppStateErrorContext {
    fn from(e: AppStateError) -> Self {
        // An error is being displayed to the user, make sure it's also written in the logs
        e.log();
        Self {
            state: AppStateErrorContextInner { errors: vec![e] },
        }
    }
}

impl IntoResponse for AppStateError {
    fn into_response(self) -> Response {
        let error_context = AppStateErrorContext::from(self);
        (StatusCode::INTERNAL_SERVER_ERROR, error_context).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_extract_from_any_error() {
        let res: Result<_, AppStateError> =
            std::fs::read("/tmp/qsjlkdjsqlkdsqfsqhsjklfhalkjfkjh.toml")
                .boxed()
                .context(OtherSnafu);

        assert!(res.is_err());
        assert_eq!(&res.unwrap_err().to_string(), "An other error occurred");
    }
}
