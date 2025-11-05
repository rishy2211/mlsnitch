use axum::{Json, http::StatusCode};
use serde::Serialize;

/// Simple health-check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// `GET /health`
///
/// Returns a basic JSON document indicating liveness.
pub async fn health() -> (StatusCode, Json<HealthResponse>) {
    (StatusCode::OK, Json(HealthResponse { status: "ok" }))
}
