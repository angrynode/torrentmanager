use hightorrent_api::hightorrent::{SingleTarget, TorrentContent, TorrentList};
use hightorrent_api::{Api, QBittorrentClient};
use migration::{Migrator, MigratorTrait};
use sea_orm::*;
use snafu::prelude::*;

use crate::config::AppConfig;

pub mod error;
pub mod free_space;

use error::*;
use free_space::FreeSpace;

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

    // TODO: multiple torrent backends
    pub torrent_client: QBittorrentClient,
}

/// Basic templating context used across pages.
///
/// Loading it may fail for some reasons, but it's so rare
/// and unrecoverable that it will trigger a global error
/// by rendering the AppStateError into an axum Response.
pub struct AppStateContext {
    pub errors: Vec<AppStateError>,
    // pub errors: Vec<String>,
    pub free_space: FreeSpace,
}

impl AppStateContext {
    fn from_app_state(state: &AppState) -> Result<Self, AppStateError> {
        Ok(Self {
            errors: vec![],
            free_space: state.free_space()?,
        })
    }
}

impl AppState {
    pub async fn context(&self) -> Result<AppStateContext, AppStateError> {
        AppStateContext::from_app_state(self)
    }

    pub async fn new(config: AppConfig) -> Result<Self, AppStateError> {
        // TODO: config for torrent backend

        let torrent_client = QBittorrentClient::new_not_logged_in(
            &config.qbittorrent_config.web_url,
            &config.qbittorrent_config.username,
            &config.qbittorrent_config.password,
        )
        .context(InitAPISnafu)?;

        let sqlite_path = config.sqlite_path.clone();
        // TODO: dehardcode
        let database = Database::connect(format!("sqlite://{}?mode=rwc", &sqlite_path))
            .await
            .context(SqliteSnafu)?;
        Migrator::up(&database, None).await.unwrap();

        Ok(Self {
            config,
            database,
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
