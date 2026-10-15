use tracing_subscriber::{filter::ParseError, reload, util::TryInitError};

/// Errors from telemetry setup and runtime filter control.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    /// Filter directives failed to parse.
    #[error("invalid filter directives")]
    Filter(#[from] ParseError),
    /// The live filter could not be read or replaced.
    #[error("failed to reload log filter")]
    Reload(#[from] reload::Error),
    /// A global subscriber was already installed.
    #[error("failed to install global subscriber")]
    Install(#[from] TryInitError),
}
