//! The magpie server binary.
mod config;

use std::future;
use std::io::Write as _;
use std::net::SocketAddr;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;

use config::{Cli, Command, Config};
use magpie_http::serve;

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            magpie_telemetry::report_fatal(&err);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load(&cli)?;
    match cli.command {
        Command::Config => run_config_command(&config)?,
        Command::Serve => run_serve_command(&config).await?,
    }

    Ok(())
}

fn run_config_command(config: &Config) -> anyhow::Result<()> {
    std::io::stdout().write_all(toml::to_string_pretty(&config)?.as_bytes())?;
    Ok(())
}

async fn run_serve_command(config: &Config) -> anyhow::Result<()> {
    let _log_control = magpie_telemetry::init(&config.log)?;
    magpie_telemetry::install_panic_hook();

    let bind_address = SocketAddr::new(config.http.ip, config.http.port);
    let listener = TcpListener::bind(bind_address)
        .await
        .with_context(|| format!("failed to bind {bind_address}"))?;
    tracing::info!(%bind_address, "listening");

    let shutdown = async {
        // If the Ctrl-C handler cannot be installed, run until killed.
        if signal::ctrl_c().await.is_err() {
            future::pending::<()>().await;
        }
        tracing::info!("shutdown signal received");
    };

    serve(listener, shutdown).await?;
    tracing::info!("shutdown complete");
    Ok(())
}
