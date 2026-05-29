use snafu::prelude::*;

use crate::state::error::AppStateError;
use crate::state::logger::LoggerError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ContentFolderError {
    #[snafu(display("There is already a content folder called `{name}` in the current folder."))]
    NameTaken { name: String },
    #[snafu(display("The folder name is invalid. It must not contain slashes."))]
    NameInvalid,
    #[snafu(display("The folder path must appear absolute"))]
    PathInvalid,
    #[snafu(display("Folder {id} does not exist."))]
    NotFound { id: i32 },
    #[snafu(display("The content folder id is invalid: {id}"))]
    IDInvalid { id: String },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
    #[snafu(display("Failed to create the folder on disk"))]
    IO { source: std::io::Error },
    #[snafu(display("Failed to load the torrent {id} requested to be moved"))]
    MovingTorrent {
        id: i32,
        source: crate::database::torrent::TorrentError,
    },
}

impl From<ContentFolderError> for AppStateError {
    fn from(e: ContentFolderError) -> Self {
        Self::ContentFolder { source: e }
    }
}
