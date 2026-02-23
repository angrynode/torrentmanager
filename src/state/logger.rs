use camino::Utf8PathBuf;
use snafu::prelude::*;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::RwLock;

use std::io::SeekFrom;
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
    #[snafu(display("Other IO error with operaitons log {path}"))]
    IO {
        path: Utf8PathBuf,
        source: std::io::Error,
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
        handle.write(b"\n").await.context(AppendSnafu {
            path: self.path.to_path_buf(),
        })?;

        handle.flush().await.context(IOSnafu {
            path: self.path.to_path_buf(),
        })?;

        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<OperationLog>, LoggerError> {
        // When in append mode, the cursor may be set to the end of file
        // so start again from the beginning
        let mut s = String::new();
        {
            let mut handle = self.handle.write().await;
            handle.seek(SeekFrom::Start(0)).await.context(IOSnafu {
                path: self.path.to_path_buf(),
            })?;

            handle.read_to_string(&mut s).await.context(ReadSnafu {
                path: self.path.to_path_buf(),
            })?;
        }

        // Now that we have dropped the RwLock, parse the results
        s.lines()
            .map(|entry| {
                serde_json::from_str(entry).context(ParseSnafu {
                    path: self.path.to_path_buf(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use async_tempfile::TempFile;
    use camino::Utf8PathBuf;
    use chrono::Utc;
    use tokio::task::JoinSet;

    use super::*;
    use crate::database::operation::*;
    use crate::extractors::user::User;
    use crate::routes::category::CategoryForm;

    #[tokio::test]
    async fn many_writers() {
        let mut set = JoinSet::new();
        let tmpfile =
            Utf8PathBuf::from_path_buf(TempFile::new().await.unwrap().file_path().to_path_buf())
                .unwrap();
        let logger = Logger::new(tmpfile.clone()).await.unwrap();

        let operation_log = OperationLog {
            user: Some(User("foo".to_string())),
            date: Utc::now(),
            table: Table::Category,
            operation: OperationType::Create,
            operation_id: OperationId {
                name: "object".to_string(),
                object_id: 1,
            },
            operation_form: Some(Operation::Category(CategoryForm {
                name: "object".to_string(),
                path: "/path".to_string(),
            })),
        };

        for _i in 0..100 {
            let logger = logger.clone();
            let operation_log = operation_log.clone();
            set.spawn(async move { logger.write(operation_log).await });
        }

        for task in set.join_all().await {
            assert!(task.is_ok());
        }

        let s = tokio::fs::read_to_string(&tmpfile).await.unwrap();
        println!("{s}");

        let logs = logger.read().await.unwrap();
        assert_eq!(logs.len(), 100);
    }

    #[tokio::test]
    async fn mixed_readers_writers() {
        let mut set = JoinSet::new();
        let tmpfile =
            Utf8PathBuf::from_path_buf(TempFile::new().await.unwrap().file_path().to_path_buf())
                .unwrap();
        let logger = Logger::new(tmpfile.clone()).await.unwrap();

        let operation_log = OperationLog {
            user: Some(User("foo".to_string())),
            date: Utc::now(),
            table: Table::Category,
            operation: OperationType::Create,
            operation_id: OperationId {
                name: "object".to_string(),
                object_id: 1,
            },
            operation_form: Some(Operation::Category(CategoryForm {
                name: "object".to_string(),
                path: "/path".to_string(),
            })),
        };

        for i in 0..200 {
            let logger = logger.clone();
            if i % 2 == 0 {
                let operation_log = operation_log.clone();
                set.spawn(async move { logger.write(operation_log).await });
            } else {
                set.spawn(async move {
                    let _ = logger.read().await?;
                    Ok(())
                });
            }
        }

        for task in set.join_all().await {
            assert!(task.is_ok());
        }

        let s = tokio::fs::read_to_string(&tmpfile).await.unwrap();
        println!("{s}");

        let logs = logger.read().await.unwrap();
        assert_eq!(logs.len(), 100);
    }
}
