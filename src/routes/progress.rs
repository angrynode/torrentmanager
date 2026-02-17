use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Path, State};
use hightorrent_api::hightorrent::{SingleTarget, Torrent, TorrentContent};

use crate::extractors::torrent_list::{
    TorrentListCounter, TorrentListFilter, TorrentListView, TorrentListViewRequest,
};
use crate::state::{AppState, AppStateContext, error::AppStateError};

#[derive(Template, WebTemplate)]
#[template(path = "progress.html")]
pub struct TorrentListTemplate {
    /// Global application state (errors/warnings)
    state: AppStateContext,
    /// Specific context for torrent lists.
    torrent_list: TorrentListContext,
    // Filter object
    filter: TorrentListViewRequest,
}

#[derive(Debug)]
pub struct TorrentListContext {
    /// Number of torrents in each state (ongoing/stuck/etc)
    pub counter: TorrentListCounter,
    /// Files associated with a specific torrent.
    ///
    /// This field is Some() only when a specific torrent is selected.
    pub files: Option<Vec<TorrentContent>>,
    /// List of selected torrents.
    ///
    /// Can be a single entry when a specific torrent was requested.
    pub torrents: Vec<Torrent>,
}

pub async fn progress(
    app_state_context: AppStateContext,
    State(app_state): State<AppState>,
    Path(view_request): Path<TorrentListViewRequest>,
) -> Result<TorrentListTemplate, AppStateError> {
    // Failing to load the TorrentListView is a fatal error
    let TorrentListView {
        counter,
        filtered_list,
    } = TorrentListView::apply_request(view_request.clone(), &app_state).await?;

    // If only one torrent is inspected, display the content files
    let files = if filtered_list.len() == 1 {
        let torrent_id = &filtered_list.first().unwrap().id;
        Some(
            app_state
                .torrent_get_files(&SingleTarget::from(torrent_id))
                .await?,
        )
    } else {
        None
    };

    Ok(TorrentListTemplate {
        state: app_state_context,
        filter: view_request,
        torrent_list: TorrentListContext {
            counter,
            files,
            torrents: filtered_list,
        },
    })
}
