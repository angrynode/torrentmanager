use sea_orm_migration::prelude::*;

use super::m20251113_203047_add_content_folder::ContentFolder;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("content_folder_uniq_path")
                    .table(ContentFolder::Table)
                    .col(ContentFolder::Path)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("content_folder_uniq_path")
                    .table(ContentFolder::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
