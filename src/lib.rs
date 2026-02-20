use axum::Router;
use axum::routing::{get, post};
use axum::serve::Listener;

pub mod config;
pub mod database;
pub mod extractors;
pub mod middleware;
pub mod migration;
pub mod routes;
pub mod state;

use state::error::AppStateError;

pub fn router(state: state::AppState) -> Router {
    Router::new()
        // Register dynamic routes
        .route("/", get(routes::index::index))
        .route("/upload", get(routes::index::upload))
        .route("/progress/{view_request}", get(routes::progress::progress))
        .route("/categories", post(routes::category::create))
        .route("/categories/new", get(routes::category::new))
        .route("/categories/{id}/delete", get(routes::category::delete))
        .route("/folders/{category_id}", get(routes::category::show))
        .route(
            "/folders/{category_name}/{*folder_path}",
            get(routes::content_folder::show),
        )
        .route("/folders", post(routes::content_folder::create))
        .route("/logs", get(routes::logs::index))
        // Register static assets routes
        .nest("/assets", torrentmanager_assets::static_router())
        // Insert request timing
        .layer(axum::middleware::from_fn(middleware::timing::add_timing))
        // Allow to access global AppState from routes
        .with_state(state)
}

pub async fn serve<L>(listener: L, config: config::AppConfig) -> Result<(), AppStateError>
where
    L: Listener,
    L::Addr: std::fmt::Debug,
{
    let state = state::AppState::new(config).await?;
    let app = router(state);

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
