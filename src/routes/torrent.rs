use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Form, Path};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use axum_typed_multipart::{TryFromMultipart, TypedMultipart};
use serde::Deserialize;
use snafu::prelude::*;

use crate::database::content_folder::FolderView;
use crate::database::torrent;
use crate::routes::content_folder::ContentFolderShowTemplate;
use crate::state::flash_message::{FallibleTemplate, OperationStatus, StatusCookie};
use crate::state::{AppStateContext, error::*};

/// POST multipart form submitted to /folders/ID/upload_torrent.
///
/// At this point, we don't want to do all the sanity checks,
/// simply ensure the data is well-formed to further process
/// the request.
#[derive(Clone, Debug, TryFromMultipart)]
pub struct TorrentForm {
    // axum_typed_multipart doesn't work with Vec<u8> with non-UTF8
    // content, see https://github.com/murar8/axum_typed_multipart/issues/88
    pub file: axum::body::Bytes,
}

/// POST form submitted to /folders/ID/upload_magnet
#[derive(Clone, Debug, Deserialize)]
pub struct MagnetForm {
    pub magnet: String,
}

pub async fn upload_torrent(
    context: AppStateContext,
    Path(id): Path<i32>,
    jar: CookieJar,
    TypedMultipart(form): TypedMultipart<TorrentForm>,
) -> Result<Response, AppStateError> {
    let view = FolderView::from_id(&context.db.content_folder(), id, None).await?;

    if let Err(e) = context
        .db
        .torrent()
        .import_torrent(view.folder.clone().unwrap(), form.file)
        .await
        .context(TorrentSnafu)
    {
        let mut template = ContentFolderShowTemplate::new(context, view);
        template.with_optional_flash(Some(OperationStatus::error(e)));
        return Ok(template.into_response());
    }

    let status = StatusCookie::success(
        jar,
        "The torrent has been uploaded and is now awaiting confirmation".to_string(),
    );
    Ok(status.redirect("/torrent").into_response())
}

pub async fn upload_magnet(
    context: AppStateContext,
    Path(id): Path<i32>,
    jar: CookieJar,
    Form(form): Form<MagnetForm>,
) -> Result<Response, AppStateError> {
    let view = FolderView::from_id(&context.db.content_folder(), id, None).await?;

    if let Err(e) = context
        .db
        .torrent()
        .import_magnet(view.folder.clone().unwrap(), form.magnet)
        .await
        .context(TorrentSnafu)
    {
        let mut template = ContentFolderShowTemplate::new(context, view);
        template.with_optional_flash(Some(OperationStatus::error(e)));
        return Ok(template.into_response());
    }

    let status = StatusCookie::success(
        jar,
        "The magnet has been uploaded and is now resolving".to_string(),
    );
    Ok(status.redirect("/torrent").into_response())
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
