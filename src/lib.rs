use axum::Router;
use axum::routing::{get, post};
use axum::serve::Listener;
use static_serve::embed_assets;

pub mod config;
pub mod database;
pub mod extractors;
pub mod filesystem;
pub mod middleware;
pub mod migration;
pub mod routes;
pub mod state;

use state::error::AppStateError;

pub fn router(state: state::AppState) -> Router {
    // Embed the assets in the binary, generating the static_router function
    embed_assets!("assets", allow_unknown_extensions = true);

    Router::new()
        // Register dynamic routes
        .route("/", get(routes::index::index))
        .route("/progress/{view_request}", get(routes::progress::progress))
        .route("/categories", post(routes::category::create))
        .route("/categories/new", get(routes::category::new))
        .route("/categories/{id}/delete", get(routes::category::delete))
        .route("/folders/{category_id}", get(routes::category::show))
        .route(
            "/folders/{category_id}",
            post(routes::category::post_magnet),
        )
        .route(
            "/folders/{category_name}/{*folder_path}",
            get(routes::content_folder::show),
        )
        .route("/folders", get(routes::index::index))
        .route(
            "/folders/{category_name}/{*folder_path}",
            post(routes::content_folder::post_magnet),
        )
        .route("/folders", post(routes::content_folder::create))
        .route("/logs", get(routes::logs::index))
        .route("/magnet", get(routes::magnet::list))
        // Register static assets routes
        .nest("/assets", static_router())
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
