use askama::Template;
use askama_web::WebTemplate;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use sea_orm::LoaderTrait;

use crate::database::{content_folder, magnet};
use crate::state::{AppStateContext, error::*};

/// Multipart form submitted to /magnet/upload:
///
/// - magnet: the magnet link to upload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagnetForm {
    pub content_folder_id: String,
    pub magnet: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "magnet/list.html")]
pub struct MagnetListTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Magnets stored in database
    pub magnets: Vec<(magnet::Model, content_folder::Model)>,
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
    let content_folders: Vec<content_folder::Model> = magnets
        .load_one(content_folder::Entity, &context.state.database)
        .await
        .context(SqliteSnafu)?
        .into_iter()
        .map(|x| x.unwrap())
        .collect();

    let magnets = magnets
        .into_iter()
        .zip(content_folders.into_iter())
        .collect();

    Ok(MagnetListTemplate {
        state: context,
        magnets,
    })
}
