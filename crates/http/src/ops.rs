//! Operations routes
use std::time::Duration;

use axum::{
    Router,
    extract::{FromRef, State},
    routing::get,
};
use hyper::StatusCode;
use tower_http::timeout::TimeoutLayer;

use magpie_db::Db;

const OPS_TIMEOUT: Duration = Duration::from_secs(30);

/// Operations routes
pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Db: FromRef<S>,
{
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            OPS_TIMEOUT,
        ))
}

async fn ready(State(db): State<Db>) -> StatusCode {
    match db.ping().await {
        Ok(()) => StatusCode::OK,
        Err(err) => {
            let err = anyhow::Error::from(err);
            tracing::warn!(error = format!("{err:#}"), "readiness check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

async fn live() -> StatusCode {
    StatusCode::OK
}
