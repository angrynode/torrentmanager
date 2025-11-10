use sea_orm::entity::prelude::*;
use snafu::prelude::*;

/// A category to store associated files.
///
/// Each category has a name and an associated path on disk, where
/// symlinks to the content will be created.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "category")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub path: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CategoryError {
    #[snafu(display("There is already a category called `{name}`"))]
    NameTaken { name: String },
    #[snafu(display("There is already a category in dir `{path}`"))]
    PathTaken { path: String },
    #[snafu(display("The parent directory does not exist: {path}"))]
    ParentDir { path: String },
    #[snafu(display("Other disk error"))]
    IO { source: std::io::Error },
    #[snafu(display("Database error"))]
    DB { source: sea_orm::DbErr },
}
