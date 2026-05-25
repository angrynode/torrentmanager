use sea_orm_migration::{prelude::*, schema::*};

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
                    .col(ColumnDef::new(ContentFolder::ParentId).integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-content-folder-parent_id")
                            .from(ContentFolder::Table, ContentFolder::ParentId)
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
    ParentId,
}
