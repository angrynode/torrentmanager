use hightorrent_api::hightorrent::{MagnetLinkError, TorrentFileError, TorrentID};
use snafu::prelude::*;

use crate::state::logger::LoggerError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum TorrentError {
    #[snafu(display("Submitted torrent file is invalid"))]
    InvalidFile { source: TorrentFileError },
    #[snafu(display("The magnet is invalid"))]
    InvalidLink { source: MagnetLinkError },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
    #[snafu(display("The torrent (ID: {id}) does not exist"))]
    NotFound { id: i32 },
    #[snafu(display("The torrent (TorrentID: {id}) does not exist"))]
    NotFoundTorrentID { id: TorrentID },
    #[snafu(display("Failed to save the operation log"))]
    Logger { source: LoggerError },
    #[snafu(display("Requested content folder not found"))]
    NoSuchContentFolder { id: i32 },
    #[snafu(display("The torrent ID {torrent_id} is already imported"))]
    Duplicate { torrent_id: TorrentID },
}
