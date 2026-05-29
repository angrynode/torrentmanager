use serde::{Deserialize, Serialize};

use crate::database::operation::Operation;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TorrentOperation {
    ImportTorrent {
        id: i32,
        name: String,
        folder: (i32, String),
    },
    ImportMagnet {
        id: i32,
        name: String,
        folder: (i32, String),
    },
    ResolveMagnet {
        id: i32,
        name: String,
    },
    MoveTorrent {
        id: i32,
        name: String,
        previous_folder: (i32, String),
        new_folder: (i32, String),
    },
}

impl From<TorrentOperation> for Operation {
    fn from(o: TorrentOperation) -> Self {
        Self::Torrent(o)
    }
}
