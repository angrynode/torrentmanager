use sea_orm_migration::{prelude::*, schema::*};

use crate::migration::m20251113_203047_add_content_folder::ContentFolder;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Torrent::Table)
                    .if_not_exists()
                    .col(pk_auto(Torrent::Id))
                    .col(string(Torrent::TorrentID).unique_key())
                    .col(var_binary(Torrent::TorrentFile, 0).null())
                    .col(string(Torrent::MagnetLink))
                    .col(string(Torrent::Name))
                    .col(integer(Torrent::Status))
                    .col(ColumnDef::new(Torrent::ContentFolderId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-magnet-content_folder_id")
                            .from(Torrent::Table, Torrent::ContentFolderId)
                            .to(ContentFolder::Table, ContentFolder::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Torrent::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Torrent {
    Table,
    Id,
    #[allow(clippy::enum_variant_names)]
    TorrentID,
    #[allow(clippy::enum_variant_names)]
    TorrentFile,
    MagnetLink,
    Name,
    Status,
    ContentFolderId,
}
