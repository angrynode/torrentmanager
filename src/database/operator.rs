use crate::database::{
    category::CategoryOperator, content_folder::ContentFolderOperator, magnet::MagnetOperator,
    torrent::TorrentOperator,
};
use crate::extractors::user::User;
use crate::state::AppState;

#[derive(Clone, Debug)]
pub struct DatabaseOperator {
    pub state: AppState,
    pub user: Option<User>,
}

impl DatabaseOperator {
    pub fn new(state: AppState, user: Option<User>) -> Self {
        Self { state, user }
    }

    pub fn category(&self) -> CategoryOperator {
        CategoryOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }

    pub fn content_folder(&self) -> ContentFolderOperator {
        ContentFolderOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }

    pub fn magnet(&self) -> MagnetOperator {
        MagnetOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }

    pub fn torrent(&self) -> TorrentOperator {
        TorrentOperator {
            state: self.state.clone(),
            user: self.user.clone(),
        }
    }
}
