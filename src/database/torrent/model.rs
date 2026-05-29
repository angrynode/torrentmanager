use hightorrent_api::hightorrent::{MagnetLink, TorrentFile, TorrentID};
use sea_orm::entity::prelude::*;

use crate::database::content_folder;

/// A category to store associated files.
///
/// Each category has a name and an associated path on disk, where
/// symlinks to the content will be created.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "torrent")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub torrent_id: TorrentID,
    pub torrent_file: Option<TorrentFile>,
    pub magnet_link: MagnetLink,
    pub name: String,
    pub content_folder_id: i32,
    #[sea_orm(belongs_to, from = "content_folder_id", to = "id")]
    pub content_folder: HasOne<content_folder::Entity>,
    pub status: TorrentStatus,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

/// The status of a torrent during the import pipeline, from
/// first upload until files are imported.
#[derive(Clone, Debug, EnumIter, DeriveActiveEnum, PartialEq)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum TorrentStatus {
    /// Magnet is resolving to a torrent
    #[sea_orm(num_value = 0)]
    Resolving,
    /// Torrent is downloading data
    #[sea_orm(num_value = 1)]
    Downloading,
    /// Torrent has finished downloading, awaiting validation to import files.
    #[sea_orm(num_value = 2)]
    Downloaded,
    /// Torrent import has been validated, symlinks are being created.
    #[sea_orm(num_value = 3)]
    Validated,
    /// Torrent is finished and imported.
    #[sea_orm(num_value = 4)]
    Imported,
}
