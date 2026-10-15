use clap::Args;
use serde::{Deserialize, Serialize};

/// Log output settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    /// Stdout sink settings.
    pub stdout: StdoutConfig,
}

/// Stdout sink settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StdoutConfig {
    /// Log line format.
    pub format: StdoutFormat,
    /// Filter directives, e.g. `info,magpie_http=debug`. `RUST_LOG` overrides this.
    pub filter: String,
}

impl Default for StdoutConfig {
    fn default() -> Self {
        Self {
            format: StdoutFormat::Auto,
            filter: "info".into(),
        }
    }
}

/// Stdout log line format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StdoutFormat {
    /// Pretty on a terminal, JSON otherwise.
    #[default]
    Auto,
    /// One JSON object per line.
    Json,
    /// Human-readable multi-line output.
    Pretty,
}

/// Log flags. Each one overrides the matching config value when given.
#[derive(Debug, Clone, Args, Serialize)]
pub struct LogArgs {
    #[command(flatten)]
    stdout: StdoutArgs,
}

#[derive(Debug, Clone, Args, Serialize)]
struct StdoutArgs {
    /// Stdout filter directives.
    #[arg(long = "log-filter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
}
