use sea_orm_migration::{prelude::*, schema::*};

use super::m20251110_01_create_table_category::Category;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ContentFolder::Table)
                    .if_not_exists()
                    .col(pk_auto(ContentFolder::Id))
                    .col(string(ContentFolder::Name))
                    .col(string(ContentFolder::Path))
                    .col(
                        ColumnDef::new(ContentFolder::CategoryId)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-content-file-category_id")
                            .from(ContentFolder::Table, ContentFolder::CategoryId)
                            .to(Category::Table, Category::Id),
                    )
                    .col(ColumnDef::new(ContentFolder::ParentId).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-content-folder-parent_id")
                            .from(ContentFolder::ParentId, ContentFolder::ParentId)
                            .to(ContentFolder::Table, ContentFolder::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ContentFolder::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ContentFolder {
    Table,
    Id,
    Name,
    Path,
    CategoryId,
    ParentId,
}
