use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderName, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{config::GatewayConfig, historical::HistoricalSource, transport};

pub fn build_router(state: transport::AppState) -> Router {
    let config = state.config.clone();
    let origins = config
        .allowed_origins
        .iter()
        .map(|origin| origin.parse::<HeaderValue>().expect("validated origin"))
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-request-id"),
        ]);
    Router::new()
        .route("/health/live", get(transport::route_health_live))
        .route("/health/ready", get(transport::route_health_ready))
        .route("/v1/history/bars", post(transport::route_history_bars))
        .route("/v1/symbols/resolve", post(transport::route_resolve))
        .route("/v1/symbols/search", post(transport::route_search))
        .route("/v1/datasets/{dataset}", get(transport::route_dataset))
        .route("/v1/live", axum::routing::get(transport::route_live))
        .layer(DefaultBodyLimit::max(config.http_body_max_bytes))
        .layer(cors)
        .with_state(state)
}

pub async fn run(
    history_source: Arc<dyn HistoricalSource + Send + Sync>,
    config: GatewayConfig,
) -> Result<(), std::io::Error> {
    run_with_live_key(history_source, config, None).await
}

/// Starts the gateway with an explicitly supplied server-side Databento key for
/// the opt-in live boundary.  The key is never placed in a response or log.
pub async fn run_with_live_key(
    history_source: Arc<dyn HistoricalSource + Send + Sync>,
    config: GatewayConfig,
    live_api_key: Option<String>,
) -> Result<(), std::io::Error> {
    config.validate()?;
    let state =
        transport::AppState::new_with_live_key(history_source, config.clone(), live_api_key);
    let router = build_router(state);
    let address = format!("{}:{}", config.bind_host, config.bind_port);
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await
}
