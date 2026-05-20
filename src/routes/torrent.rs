use askama::Template;
use askama_web::WebTemplate;
use axum_extra::extract::CookieJar;
use axum_typed_multipart::{TryFromMultipart, TypedMultipart};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use crate::database::torrent;
use crate::state::flash_message::{FlashRedirect, StatusCookie};
use crate::state::{AppStateContext, error::*};

/// POST form submitted to /torrent/upload.
///
/// At this point, we don't want to do all the sanity checks,
/// simply ensure the data is well-formed to further process
/// the request.
///
/// We don't want to fail if the torrent is invalid (according
/// to hightorrent), because we still want to retrieve the requested
/// category/content_folder to produce the error flash redirection.
#[derive(Clone, Debug, Serialize, Deserialize, TryFromMultipart)]
pub struct TorrentForm {
    pub category_id: i32,
    pub content_folder_id: Option<i32>,
    #[serde(skip)]
    // We don't want to archive every upload in the *logs*
    // axum_typed_multipart doesn't work with Vec<u8> with non-UTF8
    // content, see https://github.com/murar8/axum_typed_multipart/issues/88
    pub file: axum::body::Bytes,
}

pub async fn upload(
    context: AppStateContext,
    jar: CookieJar,
    TypedMultipart(form): TypedMultipart<TorrentForm>,
) -> Result<FlashRedirect, AppStateError> {
    // TODO: proper error type
    if let Err(e) = context
        .db
        .torrent()
        .create(&form)
        .await
        .context(TorrentUploadSnafu)
    {
        // TODO: if we had loaded the category/content_folder from the URL
        // here, we could perform this more easily… In the meantime, we have
        // to check manually if we have a corresponding category/content_folder
        // and redirect with a flash error.
        // RE: That would actually be really handy here. In the meantime,
        // we introduce a helper which has no good place so we put it in the AppState.
        let uri = context
            .linker
            .try_folder_url(form.category_id, form.content_folder_id)
            .await?;
        let status = StatusCookie::error(jar, e.to_string_recurse());
        return Ok(status.redirect(&uri));
    }

    let status = StatusCookie::success(
        jar,
        "The torrent has been uploaded and is now awaiting confirmation".to_string(),
    );
    Ok(status.redirect("/torrent"))
}

#[derive(Template, WebTemplate)]
#[template(path = "torrent/list.html")]
pub struct TorrentListTemplate {
    /// Global application state (errors/warnings)
    pub state: AppStateContext,
    /// Torrents stored in database
    pub torrents: Vec<torrent::ModelEx>,
}

pub async fn list(context: AppStateContext) -> Result<TorrentListTemplate, AppStateError> {
    let torrents = context
        .db
        .torrent()
        .list_with_related()
        .await
        .boxed()
        .context(OtherSnafu)?;

    Ok(TorrentListTemplate {
        state: context,
        torrents,
    })
}
