/// Errors from running the HTTP server.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// The server stopped with an I/O error.
    #[error("HTTP server failed")]
    Serve(#[from] std::io::Error),
}
