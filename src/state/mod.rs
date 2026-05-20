use hightorrent_api::hightorrent::{SingleTarget, TorrentContent, TorrentList};
use hightorrent_api::{Api, QBittorrentClient};
use sea_orm::*;
use snafu::prelude::*;

use crate::config::AppConfig;
use crate::extractors::user::User;
use crate::migration::{Migrator, MigratorTrait};

mod context;
pub use context::AppStateContext;
pub mod error;
pub mod flash_message;
pub mod free_space;
pub mod linker;
pub mod logger;

use error::*;
use free_space::FreeSpace;
use logger::Logger;

/// Global application state.
///
/// Used to perform queries against the system, and torrentmanager's
/// database. Can be safely cloned between threads (inner mutability).
#[derive(Clone, Debug)]
pub struct AppState {
    // Global configuration for TorrentManager
    pub config: AppConfig,

    /// Sqlite database
    pub database: DatabaseConnection,

    /// Append-only log for operations
    pub logger: Logger,

    // TODO: multiple torrent backends
    pub torrent_client: QBittorrentClient,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self, AppStateError> {
        // TODO: config for torrent backend

        let torrent_client = QBittorrentClient::new_not_logged_in(
            &config.qbittorrent_config.web_url,
            &config.qbittorrent_config.username,
            &config.qbittorrent_config.password,
        )
        .context(InitAPISnafu)?;

        let sqlite_path = config.sqlite_path.clone();
        let database = Database::connect(format!("sqlite://{}?mode=rwc", sqlite_path))
            .await
            .context(SqliteSnafu)?;

        Migrator::up(&database, None)
            .await
            .context(MigrationSnafu)?;

        let logger = Logger::new(config.log_path.clone())
            .await
            .context(LoggerSnafu)?;

        Ok(Self {
            config,
            database,
            logger,
            torrent_client,
        })
    }

    pub fn free_space(&self) -> Result<FreeSpace, AppStateError> {
        free_space::FreeSpace::from_path(&self.config.media_dir).context(FreeSpaceSnafu)
    }

    pub async fn torrent_list(&self) -> Result<TorrentList, AppStateError> {
        // TODO: errors
        self.torrent_client.list().await.context(APISnafu)
    }

    pub async fn torrent_get_files(
        &self,
        target: &SingleTarget,
    ) -> Result<Vec<TorrentContent>, AppStateError> {
        // TODO: errors
        self.torrent_client
            .get_files(target)
            .await
            .context(APISnafu)
    }
}
