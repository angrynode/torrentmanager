use askama::Template;
use askama_web::WebTemplate;
use axum::extract::Form;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::magnet;
use crate::state::flash_message::{FlashRedirect, StatusCookie};
use crate::state::{AppStateContext, error::*};

/// POST form submitted to /magnet/upload:
///
/// - magnet: the magnet link to upload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagnetForm {
    pub category_id: i32,
    pub content_folder_id: Option<i32>,
    pub magnet: String,
}

pub async fn upload(
    context: AppStateContext,
    jar: CookieJar,
    Form(form): Form<MagnetForm>,
) -> Result<FlashRedirect, AppStateError> {
    // TODO: proper error type
    if let Err(e) = context
        .db
        .magnet()
        .create(&form)
        .await
        .context(MagnetUploadSnafu)
    {
        // TODO: if we had loaded the category/content_folder from the URL
        // here, we could perform this more easily… In the meantime, we have
        // to check manually if we have a corresponding category/content_folder
        // and redirect with a flash error.
        let category = context
            .db
            .category()
            .find_by_id(form.category_id)
            .await
            .context(CategorySnafu)?;

        let status = StatusCookie::error(jar, e.to_string_recurse());

        let uri = if let Some(content_folder_id) = form.content_folder_id {
            let content_folder = context
                .db
                .content_folder()
                .find_by_id(content_folder_id)
                .await
                .context(ContentFolderSnafu)?;

            format!("/folders/{}{}", category.name, content_folder.path)
        } else {
            format!("/folders/{}", category.name)
        };

        return Ok(status.redirect(&uri));
    }

    let status = StatusCookie::success(
        jar,
        "The magnet has been uploaded and is now resolving".to_string(),
    );
    Ok(status.redirect("/magnet"))
}

#[derive(Template, WebTemplate)]
#[template(path = "magnet/list.html")]
pub struct MagnetListTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Magnets stored in database
    pub magnets: Vec<magnet::ModelEx>,
}

pub async fn list(context: AppStateContext) -> Result<MagnetListTemplate, AppStateError> {
    let magnets = context
        .db
        .magnet()
        .list_with_related()
        .await
        .boxed()
        .context(OtherSnafu)?;

    Ok(MagnetListTemplate {
        state: context,
        magnets,
    })
}
