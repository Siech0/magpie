//! Logging, tracing, and metrics: subscriber setup, runtime filter control, and the in-memory log hub.

mod config;
mod control;
mod error;

pub use config::{LogArgs, LogConfig, StdoutConfig, StdoutFormat};
pub use control::LogControl;
pub use error::TelemetryError;

use std::io::IsTerminal as _;

use tracing::{Subscriber, subscriber::NoSubscriber};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt as _, reload,
    util::SubscriberInitExt as _,
};

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// Installs the global subscriber. Must be called before any logs.
///
/// # Errors
///
/// Returns [`TelemetryError::Filter`] if the filter directives are invalid, or
/// [`TelemetryError::Install`] if a global subscriber is already installed.
pub fn init(config: &LogConfig) -> Result<LogControl, TelemetryError> {
    let stdout = &config.stdout;
    let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| stdout.filter.clone());
    let pretty = match stdout.format {
        StdoutFormat::Json => false,
        StdoutFormat::Pretty => true,
        StdoutFormat::Auto => std::io::stdout().is_terminal(),
    };
    let (subscriber, control) = build(pretty, directives, std::io::stdout)?;
    subscriber.try_init()?;

    Ok(control)
}

/// Build the subscriber and controller for the telemetry system
fn build<W>(
    pretty: bool,
    directives: String,
    writer: W,
) -> Result<(impl Subscriber + Send + Sync, LogControl), TelemetryError>
where
    W: for<'w> fmt::MakeWriter<'w> + Send + Sync + 'static,
{
    let (filter, handle) = reload::Layer::new(parse_filter(&directives)?);
    let stdout: BoxedLayer = if pretty {
        fmt::layer().pretty().with_writer(writer).boxed()
    } else {
        fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_writer(writer)
            .boxed()
    };

    let layers: Vec<BoxedLayer> = vec![stdout.with_filter(filter).boxed()];
    Ok((
        tracing_subscriber::registry().with(layers),
        LogControl::new(handle, directives),
    ))
}

fn parse_filter(directives: &str) -> Result<EnvFilter, TelemetryError> {
    Ok(EnvFilter::builder().parse(directives)?)
}

/// Report fatal log error, handling if no subscriber is present at time of error.
pub fn report_fatal(err: &anyhow::Error) {
    if tracing::dispatcher::get_default(tracing::Dispatch::is::<NoSubscriber>) {
        tracing::subscriber::with_default(fmt().json().finish(), || {
            tracing::error!(error = %format_args!("{err:#}"), "fatal");
        });
    } else {
        tracing::error!(error = %format_args!("{err:#}"), "fatal");
    }
}

/// Install panic hook for logging errors.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| tracing::error!(panic = %info, "panic")));
}
