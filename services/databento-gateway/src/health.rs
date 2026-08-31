use axum::{response::IntoResponse, Json};
use serde_json::json;

pub async fn live() -> impl IntoResponse {
    (axum::http::StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub async fn ready() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        Json(json!({ "status": "ready" })),
    )
}
