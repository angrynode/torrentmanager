use askama::Template;
use askama_web::WebTemplate;
use hightorrent_api::hightorrent::MagnetLink;
use itertools::multizip;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use sea_orm::LoaderTrait;

use crate::database::{category, content_folder, magnet, operator::DatabaseOperator};
use crate::state::{AppStateContext, error::*};

/// Multipart form submitted to /magnet/upload:
///
/// - magnet: the magnet link to upload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagnetForm {
    pub category_id: String,
    pub content_folder_id: Option<String>,
    pub magnet: String,
}

#[derive(Clone, Debug)]
pub struct ValidatedMagnetForm {
    pub category: category::Model,
    pub content_folder: Option<content_folder::Model>,
    pub magnet: MagnetLink,
}

impl ValidatedMagnetForm {
    pub async fn from_form(
        f: &MagnetForm,
        db: &DatabaseOperator,
    ) -> Result<Self, magnet::MagnetError> {
        let MagnetForm {
            category_id,
            content_folder_id,
            magnet,
        } = f;

        let magnet = MagnetLink::new(magnet).context(magnet::InvalidMagnetSnafu)?;
        let category = db
            .category()
            .find_by_id_str(category_id)
            .await
            .context(magnet::CategorySnafu)?;

        let content_folder = if let Some(content_folder_id) = content_folder_id {
            if content_folder_id.is_empty() {
                None
            } else {
                Some(
                    db.content_folder()
                        .find_by_id_str(content_folder_id)
                        .await
                        .context(magnet::ContentFolderSnafu)?,
                )
            }
        } else {
            None
        };

        Ok(Self {
            category,
            content_folder,
            magnet,
        })
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "magnet/list.html")]
pub struct MagnetListTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Magnets stored in database
    pub magnets: Vec<(
        magnet::Model,
        category::Model,
        Option<content_folder::Model>,
    )>,
}

pub async fn list(context: AppStateContext) -> Result<MagnetListTemplate, AppStateError> {
    let magnets = context
        .db
        .magnet()
        .list()
        .await
        .boxed()
        .context(OtherSnafu)?;

    // In the creation form we guarantee to set the content_folder so we can unwrap
    let content_folders: Vec<Option<content_folder::Model>> = magnets
        .load_one(content_folder::Entity, &context.state.database)
        .await
        .context(SqliteSnafu)?;

    let categories: Vec<category::Model> = magnets
        .load_one(category::Entity, &context.state.database)
        .await
        .context(SqliteSnafu)?
        .into_iter()
        .map(|x| x.unwrap())
        .collect();

    let magnets = multizip((magnets, categories, content_folders)).collect();

    Ok(MagnetListTemplate {
        state: context,
        magnets,
    })
}
