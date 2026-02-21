use crate::database::{category, content_folder};

#[derive(Clone, Debug)]
pub struct FileSystemEntry {
    pub name: String,
    pub extra: Option<String>,
    pub folder_path: String,
}

impl FileSystemEntry {
    pub fn from_category(category: &category::Model) -> Self {
        Self {
            name: category.name.to_string(),
            extra: Some(category.path.to_string()),
            folder_path: category.name.to_string(),
        }
    }

    pub fn from_categories(categories: &[category::Model]) -> Vec<Self> {
        categories.iter().map(Self::from_category).collect()
    }

    pub fn from_content_folder(
        category: &category::Model,
        content_folder: &content_folder::Model,
    ) -> Self {
        Self {
            name: content_folder.name.to_string(),
            extra: None,
            folder_path: format!("{}{}", category.name, content_folder.path),
        }
    }

    pub fn from_content_folders(
        category: &category::Model,
        content_folders: &[content_folder::Model],
    ) -> Vec<Self> {
        content_folders
            .iter()
            .map(|x| Self::from_content_folder(category, x))
            .collect()
    }
}
