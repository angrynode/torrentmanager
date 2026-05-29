use super::*;

/// A loaded folder, with all surrounding entities loaded as well:
///
/// - parents/ancestors, from the topmost to the closest
/// - direct children (non-recursive)
///
/// On the index page, `folder` is not populated.
#[derive(Clone, Debug)]
pub struct FolderView {
    pub ancestors: Vec<Model>,
    pub children: Vec<Model>,
    pub folder: Option<Model>,
}

impl FolderView {
    /// Loads the top-most folder view, which is not a folder and may not have parents.
    pub async fn index(operator: &ContentFolderOperator<'_>) -> Result<Self, ContentFolderError> {
        let children = operator
            .list()
            .await?
            .into_iter()
            .filter(|x| x.parent_id.is_none())
            .collect();

        Ok(Self {
            ancestors: vec![],
            folder: None,
            children,
        })
    }

    /// Loads a folder view for a requested folder, if it exists.
    ///
    /// Fails when:
    ///
    /// - the requested ID does not exist
    // TODO: optimize with custom query
    pub async fn from_id(
        operator: &ContentFolderOperator<'_>,
        id: i32,
    ) -> Result<Self, ContentFolderError> {
        let list = operator.list().await?;

        if let Some(folder) = list.iter().find(|x| x.id == id) {
            Ok(Self {
                ancestors: folder
                    .ancestors_from_list(&list)
                    .into_iter()
                    .cloned()
                    .collect(),
                children: folder
                    .children_from_list(&list)
                    .into_iter()
                    .cloned()
                    .collect(),
                folder: Some(folder.clone()),
            })
        } else {
            Err(ContentFolderError::NotFound { id })
        }
    }
}
