use crate::database::content_folder::ContentFolderOperator;
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

    pub fn content_folder<'a>(&'a self) -> ContentFolderOperator<'a> {
        ContentFolderOperator { db: self }
    }
}
