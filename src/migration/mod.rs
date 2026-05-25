pub use sea_orm_migration::prelude::*;

mod m20251113_203047_add_content_folder;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20251113_203047_add_content_folder::Migration)]
    }
}
