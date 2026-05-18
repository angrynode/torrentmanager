use sea_orm_migration::{prelude::*, schema::*};

use crate::migration::m20251110_01_create_table_category::Category;
use crate::migration::m20251113_203047_add_content_folder::ContentFolder;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Magnet::Table)
                    .if_not_exists()
                    .col(pk_auto(Magnet::Id))
                    .col(string(Magnet::TorrentID).unique_key())
                    .col(string(Magnet::Name))
                    .col(string(Magnet::Link))
                    .col(ColumnDef::new(Magnet::ContentFolderId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-magnet-content_folder_id")
                            .from(Magnet::Table, Magnet::ContentFolderId)
                            .to(ContentFolder::Table, ContentFolder::Id),
                    )
                    .col(ColumnDef::new(Magnet::CategoryId).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-magnet-category_id")
                            .from(Magnet::Table, Magnet::CategoryId)
                            .to(Category::Table, Category::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Magnet::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Magnet {
    Table,
    Id,
    TorrentID,
    Name,
    Link,
    ContentFolderId,
    CategoryId,
}
