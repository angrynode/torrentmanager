pub use sea_orm_migration::prelude::*;

mod m20251110_01_create_table_category;
mod m20251113_203047_add_content_folder;
mod m20251113_203899_add_uniq_to_content_folder;
mod m20251114_01_create_table_magnet;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20251110_01_create_table_category::Migration),
            Box::new(m20251113_203047_add_content_folder::Migration),
            Box::new(m20251113_203899_add_uniq_to_content_folder::Migration),
            Box::new(m20251114_01_create_table_magnet::Migration),
        ]
    }
}
