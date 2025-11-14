use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

use crate::extractors::user::User;
use crate::routes::category::CategoryForm;
use crate::routes::content_folder::ContentFolderForm;

/// Type of operation applied to the database.
#[derive(Clone, Debug, Display, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationId {
    pub object_id: i32,
    pub name: String,
}

#[derive(Clone, Debug, Display, Serialize, Deserialize)]
pub enum Table {
    Category,
    ContentFolder,
}

/// Operation applied to the database.
///
/// Will be saved as an [OperationLog].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operation {
    Category(CategoryForm),
    ContentFolder(ContentFolderForm),
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &serde_json::to_string(self).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationLog {
    pub user: Option<User>,
    pub date: DateTime<Utc>,
    pub table: Table,
    pub operation: OperationType,
    pub operation_id: OperationId,
    // Raw operation parameters
    pub operation_form: Option<Operation>,
}
