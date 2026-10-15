use tracing_subscriber::{Registry, filter::EnvFilter, reload};

use crate::TelemetryError;

/// Changes log filters at runtime without a restart.
#[derive(Debug, Clone)]
pub struct LogControl {
    stdout: reload::Handle<EnvFilter, Registry>,
    startup: String,
}

impl LogControl {
    pub(crate) const fn new(stdout: reload::Handle<EnvFilter, Registry>, startup: String) -> Self {
        Self { stdout, startup }
    }

    /// The active stdout filter directives.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::Reload`] if the subscriber has been dropped.
    pub fn current(&self) -> Result<String, TelemetryError> {
        Ok(self.stdout.with_current(ToString::to_string)?)
    }

    /// Replace the current stdout filter. Invalid directives leave the current filter.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::Filter`] if the directives are invalid, or
    /// [`TelemetryError::Reload`] if the subscriber has been dropped.
    pub fn set(&self, directives: &str) -> Result<(), TelemetryError> {
        self.stdout.reload(crate::parse_filter(directives)?)?;
        Ok(())
    }

    /// Restores the filter the process started with.
    ///
    /// # Errors
    ///
    /// Returns [`TelemetryError::Reload`] if the subscriber has been dropped.
    pub fn reset(&self) -> Result<(), TelemetryError> {
        self.set(&self.startup)
    }
}
