
/// Type of operation applied to the database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationId {
    pub object_id: i64,
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
pub enum Operation {
    Category(CategoryForm),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationLog {
    user: Option<User>,
    date: DateTime<Utc>,
    table: Table,
    operation: OperationType,
    operation_id: OperationId,
    // Raw operation parameters
    operation_form: CategoryForm,
}
