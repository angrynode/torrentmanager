use askama::Template;
use askama_web::WebTemplate;
use axum::extract::Form;
use axum::response::{IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::{category, magnet};
use crate::state::{AppStateContext, error::*};

/// Multipart form submitted to /magnet/upload:
///
/// - magnet: the magnet link to upload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagnetForm {
    pub magnet: String,
}

pub async fn upload(
    context: AppStateContext,
    Form(form): Form<MagnetForm>,
) -> Result<Response, AppStateError> {
    // TODO: proper error type
    if let Err(e) = context
        .db
        .magnet()
        .create(&form)
        .await
        .context(MagnetUploadSnafu)
    {
        return Ok(UploadMagnetTemplate::new(context)
            .await?
            .with_errored_form(form, e)
            .into_response());
    }

    Ok(Redirect::to("/magnet").into_response())
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
