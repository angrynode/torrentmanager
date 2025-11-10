use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;
use tokio::fs::{create_dir_all, read, try_exists, write};
use tokio_listener::ListenerAddress;
use xdg::BaseDirectories;

#[derive(Debug, Snafu)]
pub enum ConfigError {
    #[snafu(display(
        "Failed to find configuration file: {path}\nhelp: Run `torrentmanager config generate` to create a default configuration file."
    ))]
    NoXDGConfigFile { path: Utf8PathBuf },
    #[snafu(display("Failed to read configuration file: {path} (see errors below)"))]
    FailedReadConfig {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Failed to interpret configuration file: {path} (see errors below)"))]
    FailedParseConfig {
        path: Utf8PathBuf,
        source: toml::de::Error,
    },
    #[snafu(display("Failed to create configuration directory: {path} (see errors below)"))]
    FailedConfigDir {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("An unknown IO error occurred (see errors below)"))]
    FailedIO { source: std::io::Error },
}

/// TorrentManager configuration file.
///
/// By default, loaded from $XDG_CONFIG_DIR/torrentmanager/config.toml,
/// that is usually ~/.config/torrentmanager/config.toml
///
/// The media directory:
///
/// - is assumed to be in a single disk/partition for free
///   space calculation
/// - has a .sources hidden directory used by the torrent client
///   to store files for seeding
/// - contains the directories for the different categories
///
/// The qbittorent config:
/// - use to connect torrentmanager to an instance of qbittorrent
///
/// What is currently not configurable:
///
/// - where magnets/torrents uploaded to TorrentManager are stored, hardcoded
///   to $XDG_DATA_DIR/torrentmanager/uploads
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// XDG configuration to produce standard paths for config/db.
    ///
    /// This is not saved in the file.
    #[serde(skip, default = "AppConfig::xdg_base_directories")]
    pub xdg_base_directories: BaseDirectories,

    /// Path to the configuration file itself.
    ///
    /// This is not saved in the file, but added manually after loading.
    #[serde(skip, default)]
    config_path: Utf8PathBuf,

    pub qbittorrent_config: QbittorrentConfig,

    /// Main directory where content files are stored
    pub media_dir: Utf8PathBuf,

    /// IP:PORT or Unix socket path to start the server (default: `127.0.0.1:8000`).
    ///
    /// Examples:
    ///
    /// - `0.0.0.0:8000` to listen on port 8000 on all interfaces, when you have a reverse proxy
    ///   that's not on the same machine (be careful with the firewall rules)
    /// - `/run/torrentmanager/server.sock` to listen on a specific socket file; torrentmanager
    ///   has no permission to create a file in /run, so make sure your systemd service does it
    ///   for you with the `RuntimeDirectory=torrentmanager` directive
    ///
    /// TODO: settings to change permissions on the socket to allow eg. httpd group to read/write
    #[serde(default = "AppConfig::default_listener_address")]
    pub listen: ListenerAddress,

    #[serde(default = "AppConfig::default_sqlite_path")]
    pub sqlite_path: Utf8PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QbittorrentConfig {
    // Web URL of your Qbittorrent instance
    pub web_url: String,
    // Admin username for accessing qBittorrent
    pub username: String,
    // Admin password for accessing qBittorrent
    pub password: String,
}

impl AppConfig {
    pub fn xdg_base_directories() -> BaseDirectories {
        BaseDirectories::with_prefix("torrentmanager")
    }

    pub fn default_listener_address() -> ListenerAddress {
        "127.0.0.1:8000".parse().unwrap()
    }

    pub fn config_dir() -> Utf8PathBuf {
        // Will not panic unless $HOME isn't set
        let config_dir = Self::xdg_base_directories().get_config_home().unwrap();

        // Ensure we have valid UTF8 in the path
        Utf8PathBuf::from_path_buf(config_dir).unwrap()
    }

    pub fn default_sqlite_path() -> Utf8PathBuf {
        // At this point the directory has already been successfully created
        Self::config_dir().join("database.sqlite")
    }

    pub async fn load_from_xdg() -> Result<Self, ConfigError> {
        let config_dir = Self::config_dir();
        create_dir_all(&config_dir)
            .await
            .context(FailedConfigDirSnafu {
                path: config_dir.to_path_buf(),
            })?;

        let config_path = config_dir.join("config.toml");

        log::info!("Looking up default XDG configuration: {config_path}");

        if !try_exists(&config_path).await.context(FailedIOSnafu)? {
            return Err(ConfigError::NoXDGConfigFile { path: config_path });
        }

        Self::load(&config_path).await
    }

    pub async fn load(path: &Utf8Path) -> Result<Self, ConfigError> {
        log::info!("Loading config file: {path}");

        let content = read(path).await.context(FailedReadConfigSnafu {
            path: path.to_path_buf(),
        })?;
        toml::from_slice(&content).context(FailedParseConfigSnafu {
            path: path.to_path_buf(),
        })
    }

    pub async fn save(&self) {
        // TODO: errors
        let content = toml::to_string_pretty(&self).unwrap();
        write(&self.config_path, &content).await.unwrap();
    }
}
