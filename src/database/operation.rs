use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::database::content_folder;
use crate::database::torrent;
use crate::extractors::user::User;

/// Type of operation applied to the database.
#[derive(Clone, Debug, Display, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, Display, Serialize, Deserialize)]
pub enum Table {
    ContentFolder,
    Torrent,
}

/// Operation applied to the database.
///
/// Will be saved as an [OperationLog].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operation {
    ContentFolder(content_folder::ContentFolderOperation),
    Torrent(torrent::TorrentOperation),
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationLog {
    pub user: Option<User>,
    pub date: DateTime<Utc>,
    pub operation: Operation,
    pub operation_type: OperationType,
    pub table: Table,
}
