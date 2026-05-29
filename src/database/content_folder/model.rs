use camino::{Utf8Path, Utf8PathBuf};
use sea_orm::entity::prelude::*;

use crate::database::torrent;
use crate::extractors::normalized_path::NormalizedPathComponent;

/// A content folder to store associated files.
///
/// Each content folder has a name and an associated path on disk, a Category
/// and it can have an Parent Content Folder (None if it's the first folder
/// in category)
#[sea_orm::model]
#[derive(DeriveEntityModel, Clone, Debug, PartialEq, Eq)]
#[sea_orm(table_name = "content_folder")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: NormalizedPathComponent,
    pub parent_id: Option<i32>,
    #[sea_orm(self_ref, relation_enum = "Parent", from = "parent_id", to = "id")]
    pub parent: HasOne<Entity>,
    #[sea_orm(has_many)]
    pub torrents: HasMany<torrent::Entity>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {}

impl Model {
    #[allow(dead_code)]
    pub fn siblings_from_list<'a>(&self, list: &'a [Self]) -> Vec<&'a Self> {
        list.iter()
            .filter(|x| self.parent_id == x.parent_id)
            .collect()
    }

    pub fn ancestors_from_list<'a>(&self, list: &'a [Self]) -> Vec<&'a Self> {
        // At first we traverse the filetree from the bottom up we'll reverse it later.
        let mut ancestors = vec![];

        let mut parent_id = self.parent_id;
        while let Some(id) = parent_id {
            // Here we assume the parent cannot have been deleted without the relation being deleted too
            // Otherwise, it will crash!
            let parent = list.iter().find(|x| x.id == id).unwrap();
            parent_id = parent.parent_id;
            ancestors.push(parent);
        }

        ancestors.into_iter().rev().collect()
    }

    pub fn children_from_list<'a>(&self, list: &'a [Self]) -> Vec<&'a Self> {
        list.iter()
            .filter(|x| x.parent_id == Some(self.id))
            .collect()
    }

    pub fn path_from_list(&self, basedir: &Utf8Path, list: &[Self]) -> Utf8PathBuf {
        let mut path = basedir.to_path_buf();
        for folder in self.ancestors_from_list(list) {
            path.push(folder.name.as_str());
        }
        path.push(self.name.as_str());
        path
    }
}
