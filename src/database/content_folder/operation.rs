use serde::{Deserialize, Serialize};

use crate::database::operation::Operation;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContentFolderOperation {
    Create {
        id: i32,
        name: String,
        parent: Option<(i32, String)>,
    },
    UpdateName {
        id: i32,
        old_name: String,
        new_name: String,
    },
    UpdateParent {
        id: i32,
        parent: Option<(i32, String)>,
    },
    Delete {
        id: i32,
        name: String,
    },
}

impl From<ContentFolderOperation> for Operation {
    fn from(o: ContentFolderOperation) -> Self {
        Self::ContentFolder(o)
    }
}
