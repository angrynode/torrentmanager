use camino::Utf8PathBuf;
use hightorrent_api::hightorrent::{SingleTarget, TorrentContent, TorrentList};
use hightorrent_api::{Api, QBittorrentClient};
use migration::{Migrator, MigratorTrait};
use sea_orm::*;
use snafu::prelude::*;

use crate::config::AppConfig;
use crate::database::category::{self, CategoryError};

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

    /// List categories
    ///
    /// Should not fail, unless SQLite was corrupted for some reason.
    pub async fn category_list(&self) -> Result<Vec<category::Model>, AppStateError> {
        category::Entity::find()
            .all(&self.database)
            .await
            .context(SqliteSnafu)
    }

    /// Create a new category, creating the corresponding directory.
    ///
    /// Fails if:
    ///
    /// - name or path is already taken (they should be unique)
    /// - path parent directory does not exist (to avoid completely wrong paths)
    pub async fn category_create(&self, name: String, path: String) -> Result<(), AppStateError> {
        let dir = Utf8PathBuf::from(&path);
        let parent = dir.parent().unwrap();

        if !tokio::fs::try_exists(parent)
            .await
            .context(category::IOSnafu)
            .context(CategorySnafu)?
        {
            return Err(CategoryError::ParentDir {
                path: parent.to_string(),
            })
            .context(CategorySnafu);
        }

        // Check duplicates
        let list = self.category_list().await?;
        if list.iter().any(|x| x.name == name) {
            return Err(CategoryError::NameTaken { name }).context(CategorySnafu);
        }
        if list.iter().any(|x| x.path == path) {
            return Err(CategoryError::PathTaken { path }).context(CategorySnafu);
        }

        category::ActiveModel {
            name: Set(name),
            path: Set(path),
            ..Default::default()
        }
        .save(&self.database)
        .await
        .context(category::DBSnafu)
        .context(CategorySnafu)?;

        Ok(())
    }
}
