use std::net::{IpAddr, Ipv4Addr};

use clap::Args;
use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
const DEFAULT_PORT: u16 = 3000;
#[cfg(not(debug_assertions))]
const DEFAULT_PORT: u16 = 80;

/// HTTP listener settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpConfig {
    /// Address to bind.
    pub ip: IpAddr,
    /// Port to bind.
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: DEFAULT_PORT,
        }
    }
}

/// HTTP flags. Each one overrides the matching config value when given.
#[derive(Debug, Clone, Args, Serialize)]
#[command(next_help_heading = "HTTP")]
pub struct HttpArgs {
    /// Address to bind.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    ip: Option<IpAddr>,
    /// Port to bind.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
}
