//! Shared HTTP glue: application state, extractors, and error responses.
mod config;
mod error;

pub use config::{HttpArgs, HttpConfig};
pub use error::HttpError;

use std::time::Duration;

use axum::{Router, http::StatusCode, routing::get};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Serves the application on `listener` until `shutdown` completes, then drains open connections.
///
/// # Errors
///
/// Returns [`HttpError::Serve`] if the server stops with an I/O error.
pub async fn serve<Shutdown>(listener: TcpListener, shutdown: Shutdown) -> Result<(), HttpError>
where
    Shutdown: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

/// Returns the primary router
fn router() -> Router {
    Router::new()
        .route("/health/live", get(|| async { StatusCode::OK }))
        .route("/health/ready", get(|| async { StatusCode::OK }))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new())
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    REQUEST_TIMEOUT,
                )),
        )
}
