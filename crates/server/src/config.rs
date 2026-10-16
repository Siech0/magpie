use std::path::PathBuf;

use clap::{Parser, Subcommand};
use figment::Figment;
use figment::providers::{Env, Format as _, Serialized, Toml};
use magpie_db::{DbArgs, DbConfig};
use magpie_http::{HttpArgs, HttpConfig};
use magpie_telemetry::{LogArgs, LogConfig};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG: &str = "magpie.toml";

#[derive(Debug, Parser, Serialize)]
pub(crate) struct Cli {
    /// Config file path. Defaults to `magpie.toml` when present.
    #[arg(long, env = "MAGPIE_CONFIG")]
    #[serde(skip)]
    pub(crate) config: Option<PathBuf>,
    #[command(subcommand)]
    #[serde(skip)]
    pub(crate) command: Command,
    #[command(flatten)]
    pub(crate) log: LogArgs,
    #[command(flatten)]
    pub(crate) http: HttpArgs,
    #[command(flatten)]
    pub(crate) db: DbArgs,
}

#[derive(Debug, Clone, Copy, Subcommand)]
pub(crate) enum Command {
    /// Print the effective configuration as TOML and exit.
    Config,
    /// Start the server
    Serve,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) log: LogConfig,
    pub(crate) http: HttpConfig,
    pub(crate) db: DbConfig,
}

impl Config {
    /// Precedence: flags, then `MAGPIE_*` env, then the file, then defaults.
    pub(crate) fn load(cli: &Cli) -> anyhow::Result<Self> {
        let file: Option<PathBuf> = match &cli.config {
            Some(path) if path.is_dir() => {
                // Trivial and common error worthy of a guard, commonly occurring with flawed container mounts.
                if path.extension().is_some_and(|ext| ext == "toml") {
                    anyhow::bail!(
                        "config path is a directory with a .toml extension, verify contents: {}",
                        path.display()
                    );
                }
                // Attempt to use default config within directory
                let file = path.join(DEFAULT_CONFIG);
                if !file.is_file() {
                    anyhow::bail!("config file not found: {}", file.display());
                }
                Some(file)
            }
            Some(path) if !path.is_file() => {
                anyhow::bail!("config file not found: {}", path.display());
            }
            Some(path) => Some(path.clone()),
            None => {
                let file = PathBuf::from(DEFAULT_CONFIG);
                if file.is_dir() {
                    anyhow::bail!(
                        "config path is a directory with a .toml extension, verify contents: {}",
                        file.display()
                    );
                }

                file.is_file().then_some(file)
            }
        };

        let mut figment = Figment::new();

        if let Some(file) = file {
            figment = figment.merge(Toml::file_exact(file));
        }

        Ok(figment
            .merge(Env::prefixed("MAGPIE_").split("__").ignore(&["config"]))
            .merge(Serialized::defaults(cli))
            .extract()?)
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory as _;

    use super::Cli;

    #[test]
    fn cli_definition_is_valid() {
        // Flattened argument structs from several crates share one namespace. This
        // runs clap's checks for duplicate ids and flags.
        Cli::command().debug_assert();
    }
}
