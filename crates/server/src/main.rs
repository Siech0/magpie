//! The magpie server binary.
mod config;
mod routes;
mod state;

use std::future;
use std::io::Write as _;
use std::net::SocketAddr;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;

use config::{Cli, Command, Config};

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
        Command::Migrate => run_migrate_command(&config).await?,
    }

    Ok(())
}

fn run_config_command(config: &Config) -> anyhow::Result<()> {
    std::io::stdout().write_all(toml::to_string_pretty(&config)?.as_bytes())?;
    Ok(())
}

async fn run_migrate_command(config: &Config) -> anyhow::Result<()> {
    magpie_telemetry::init(&config.log)?;
    let db = magpie_db::Db::connect(&config.db).await?;
    db.migrate().await?;
    tracing::info!("database migrated");
    Ok(())
}

async fn run_serve_command(config: &Config) -> anyhow::Result<()> {
    let log_control = magpie_telemetry::init(&config.log)?;
    magpie_telemetry::install_panic_hook();

    // Connect and migrate before binding, so the port only opens once the server can answer.
    let db = magpie_db::Db::connect(&config.db).await?;
    if config.db.auto_migrate {
        db.migrate().await?;
    }

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

    let app = routes::app(state::AppState { db, log_control });
    magpie_http::serve(listener, app, shutdown).await?;
    tracing::info!("shutdown complete");

    Ok(())
}
