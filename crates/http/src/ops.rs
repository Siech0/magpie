//! Operations routes
use std::time::Duration;

use axum::{Router, routing::get};
use hyper::StatusCode;
use tower_http::timeout::TimeoutLayer;

const OPS_TIMEOUT: Duration = Duration::from_secs(30);

/// Operations routes
pub fn routes<State>() -> Router<State>
where
    State: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health/live", get(|| async { StatusCode::OK }))
        .route("/health/ready", get(|| async { StatusCode::OK }))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            OPS_TIMEOUT,
        ))
}
