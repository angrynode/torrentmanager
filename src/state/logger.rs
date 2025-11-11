use camino::Utf8PathBuf;
use snafu::prelude::*;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

use std::sync::Arc;

use crate::database::operation::OperationLog;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum LoggerError {
    /// Failed to create the log file, which did not previously exist.
    #[snafu(display("Failed to create the operations log {path}"))]
    Create {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    /// Failed to read the log file
    #[snafu(display("Failed to read the operations log {path}"))]
    Read {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    /// Failed to append a line to the log file
    #[snafu(display("Failed to append to the operations log {path}"))]
    Append {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Failed to parse JSON from operations log {path}"))]
    Parse {
        path: Utf8PathBuf,
        source: serde_json::Error,
    },
}

#[derive(Clone, Debug)]
pub struct Logger {
    /// Log file storage
    pub path: Utf8PathBuf,

    /// Handle to the log file in append-mode
    pub handle: Arc<RwLock<File>>,
}

impl Logger {
    pub async fn new(path: Utf8PathBuf) -> Result<Self, LoggerError> {
        let mut handle = OpenOptions::new();
        let mut handle = handle.append(true).read(true);
        let handle = if !tokio::fs::try_exists(&path).await.context(ReadSnafu {
            path: path.to_path_buf(),
        })? {
            handle = handle.create(true);
            handle.open(&path).await.context(CreateSnafu {
                path: path.to_path_buf(),
            })?
        } else {
            handle.open(&path).await.context(ReadSnafu {
                path: path.to_path_buf(),
            })?
        };

        Ok(Self {
            path: path.to_path_buf(),
            handle: Arc::new(RwLock::new(handle)),
        })
    }

    pub async fn write(&self, operation: OperationLog) -> Result<(), LoggerError> {
        // This should never fail
        let operation = serde_json::to_string(&operation).unwrap();

        let mut handle = self.handle.write().await;
        handle
            .write(operation.as_bytes())
            .await
            .context(AppendSnafu {
                path: self.path.to_path_buf(),
            })?;

        Ok(())
    }
}
