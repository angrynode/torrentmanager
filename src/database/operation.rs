use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::extractors::user::User;
use crate::routes::category::CategoryForm;

/// Type of operation applied to the database.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Table {
    Category,
}

/// Operation applied to the database.
///
/// Will be saved as an [OperationLog].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operation {
    Category(CategoryForm),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationLog {
    pub user: Option<User>,
    pub date: DateTime<Utc>,
    pub table: Table,
    pub operation: OperationType,
    pub operation_id: OperationId,
    // Raw operation parameters
    pub operation_form: Operation,
}
