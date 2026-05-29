use chrono::Utc;

use std::ops::Deref;

use crate::database::content_folder::ContentFolderOperator;
use crate::database::operation::{Operation, OperationLog, OperationType, Table};
use crate::database::torrent::TorrentOperator;
use crate::extractors::user::User;
use crate::state::AppState;
use crate::state::logger::LoggerError;

pub trait TableOperator: Deref<Target = DatabaseOperator> {
    fn table(&self) -> Table;

    fn log_create(
        &self,
        operation: impl Into<Operation>,
    ) -> impl std::future::Future<Output = Result<(), LoggerError>> {
        self.log(self.table(), OperationType::Create, operation.into())
    }

    fn log_update(
        &self,
        operation: impl Into<Operation>,
    ) -> impl std::future::Future<Output = Result<(), LoggerError>> {
        self.log(self.table(), OperationType::Update, operation.into())
    }

    fn log_delete(
        &self,
        operation: impl Into<Operation>,
    ) -> impl std::future::Future<Output = Result<(), LoggerError>> {
        self.log(self.table(), OperationType::Delete, operation.into())
    }
}

#[derive(Clone, Debug)]
pub struct DatabaseOperator {
    pub state: AppState,
    pub user: Option<User>,
}

impl DatabaseOperator {
    pub fn new(state: AppState, user: Option<User>) -> Self {
        Self { state, user }
    }

    pub async fn log(
        &self,
        table: Table,
        operation_type: OperationType,
        operation: Operation,
    ) -> Result<(), LoggerError> {
        let operation = OperationLog {
            user: self.user.clone(),
            date: Utc::now(),
            operation,
            operation_type,
            table,
        };

        self.state.logger.write(operation).await
    }

    pub fn content_folder<'a>(&'a self) -> ContentFolderOperator<'a> {
        ContentFolderOperator { db: self }
    }

    pub fn torrent<'a>(&'a self) -> TorrentOperator<'a> {
        TorrentOperator { db: self }
    }
}
