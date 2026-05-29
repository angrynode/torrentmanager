use snafu::prelude::*;

use crate::database::torrent;

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
    pub torrents: Vec<torrent::Model>,
    pub moving_torrent: Option<torrent::Model>,
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
            torrents: vec![],
            moving_torrent: None,
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
        moving_id: Option<i32>,
    ) -> Result<Self, ContentFolderError> {
        let list = operator.list().await?;

        if let Some(folder) = list.iter().find(|x| x.id == id) {
            let torrents = operator.torrent().list_for_folder(folder).await.unwrap();

            let moving_torrent = if let Some(moving_id) = moving_id {
                Some(
                    operator
                        .torrent()
                        .get(moving_id)
                        .await
                        .context(MovingTorrentSnafu { id: moving_id })?,
                )
            } else {
                None
            };

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
                torrents,
                moving_torrent,
            })
        } else {
            Err(ContentFolderError::NotFound { id })
        }
    }
}
