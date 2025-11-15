use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Form, Path};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::category;
use crate::database::magnet;
use crate::state::{AppStateContext, error::*};

/// Multipart form submitted to /magnet/upload:
///
/// - magnet: the magnet link to upload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagnetForm {
    pub magnet: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "magnet/show.html")]
pub struct MagnetTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Parsed magnet from form
    pub magnet: magnet::Model,
}

pub async fn show(
    context: AppStateContext,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppStateError> {
    let magnet = context
        .db
        .magnet()
        .get(id)
        .await
        .boxed()
        .context(OtherSnafu)?;

    Ok(MagnetTemplate {
        state: context,
        magnet,
    })
}

pub async fn upload(
    context: AppStateContext,
    Form(form): Form<MagnetForm>,
) -> Result<Response, AppStateError> {
    // Parse magnet
    match context
        .db
        .magnet()
        .create(&form)
        .await
        .context(MagnetUploadSnafu)
    {
        Ok(magnet_model) => Ok(MagnetTemplate {
            state: context,
            magnet: magnet_model,
        }
        .into_response()),
        Err(e) => Ok(UploadMagnetTemplate::new(context)
            .await?
            .with_errored_form(form, e)
            .into_response()),
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "magnet/list.html")]
pub struct MagnetListTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Magnets stored in database
    pub magnets: Vec<magnet::Model>,
}

pub async fn list(context: AppStateContext) -> Result<impl IntoResponse, AppStateError> {
    let magnets = context
        .db
        .magnet()
        .list()
        .await
        .boxed()
        .context(OtherSnafu)?;

    Ok(MagnetListTemplate {
        state: context,
        magnets,
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "magnet/upload.html")]
pub struct UploadMagnetTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Magnet upload form
    pub post: Option<MagnetForm>,
    /// Error with submitted magnet
    pub post_error: Option<AppStateError>,
    pub categories: Vec<category::Model>,
}

pub async fn get_upload(context: AppStateContext) -> Result<UploadMagnetTemplate, AppStateError> {
    UploadMagnetTemplate::new(context).await
}

impl UploadMagnetTemplate {
    pub async fn new(context: AppStateContext) -> Result<Self, AppStateError> {
        let categories = context.db.category().list().await.context(CategorySnafu)?;

        Ok(UploadMagnetTemplate {
            state: context,
            categories,
            post: None,
            post_error: None,
        })
    }

    pub fn with_errored_form(mut self, form: MagnetForm, error: AppStateError) -> Self {
        self.post = Some(form);
        self.post_error = Some(error);
        self
    }
}
