//! Shared HTTP glue: application state, extractors, and error responses.
mod config;
mod error;
pub mod middleware;
pub mod ops;

use axum::Router;
pub use config::{HttpArgs, HttpConfig};
pub use error::HttpError;

use tokio::net::TcpListener;

/// Serves the application on `listener` until `shutdown` completes, then drains open connections.
///
/// # Errors
///
/// Returns [`HttpError::Serve`] if the server stops with an I/O error.
pub async fn serve<Shutdown>(
    listener: TcpListener,
    app: Router,
    shutdown: Shutdown,
) -> Result<(), HttpError>
where
    Shutdown: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}
