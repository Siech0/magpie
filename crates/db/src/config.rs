use std::{path::PathBuf, time::Duration};

use clap::{Args, ValueEnum};
use magpie_core::secret::{SecretRef, SecretRefParser};
use magpie_core::validate::{non_zero, requires};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgSslMode;

#[cfg(debug_assertions)]
const DEFAULT_SQLITE_PATH: &str = "magpie.db";
#[cfg(not(debug_assertions))]
const DEFAULT_SQLITE_PATH: &str = "/var/lib/magpie/magpie.db";

/// Database settings. Exactly one backend, SQLite when none is configured.
#[derive(Debug, Default, Serialize, Deserialize, garde::Validate)]
#[serde(default, deny_unknown_fields)]
pub struct DbConfig {
    /// The selected backend.
    #[garde(dive)]
    pub backend: Backend,
    /// Pool settings for either backend.
    #[garde(dive)]
    pub pool: PoolConfig,
}

/// Connection pool settings.
#[derive(Debug, Serialize, Deserialize, garde::Validate)]
#[serde(default, deny_unknown_fields)]
pub struct PoolConfig {
    /// Maximum connections per instance.
    #[garde(range(min = 1))]
    pub max_connections: u32,
    /// Minimum database connections.
    #[garde(range(max = self.max_connections))]
    pub min_connections: u32,
    /// Close connections idle this long. Keep it under any network idle cutoff
    /// (firewalls, load balancers, `PgBouncer`).
    #[serde(with = "magpie_core::duration")]
    #[garde(custom(non_zero))]
    pub idle_timeout: Duration,
    /// Retire connections at this age to follow failovers and rotated credentials.
    #[serde(with = "magpie_core::duration")]
    #[garde(custom(non_zero))]
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 4,
            min_connections: 0,
            idle_timeout: Duration::from_mins(10),
            max_lifetime: Duration::from_mins(30),
        }
    }
}

/// The database backend, picked by the `type` key in `[db.backend]`.
#[derive(Debug, Serialize, Deserialize, garde::Validate)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Backend {
    /// A Postgres server.
    Postgres(#[garde(dive)] PostgresConfig),
    /// A local SQLite file.
    Sqlite(#[garde(dive)] SqliteConfig),
}

impl Default for Backend {
    fn default() -> Self {
        Self::Sqlite(SqliteConfig::default())
    }
}

/// Postgres connection settings.
#[derive(Debug, Serialize, Deserialize, garde::Validate)]
#[serde(default, deny_unknown_fields)]
pub struct PostgresConfig {
    /// Server host name, or a Unix socket directory.
    #[garde(length(min = 1))]
    pub host: String,
    /// Server port.
    #[garde(range(min = 1))]
    pub port: u16,
    /// Database name.
    #[garde(length(min = 1))]
    pub database: String,
    /// Role to connect as.
    #[garde(length(min = 1))]
    pub username: String,
    /// Password, if the server uses password authentication.
    #[garde(skip)]
    pub password: Option<SecretRef>,
    /// TLS policy.
    #[garde(skip)]
    pub ssl_mode: SslMode,
    /// CA certificate (PEM) used to verify the server, instead of the default roots.
    #[garde(skip)]
    pub ssl_root_cert: Option<SecretRef>,
    /// Client certificate (PEM) for mutual TLS. Requires `ssl_client_key`.
    #[garde(custom(requires(self.ssl_client_key.as_ref(), "ssl_client_key")))]
    pub ssl_client_cert: Option<SecretRef>,
    /// Client private key (PEM) for mutual TLS. Requires `ssl_client_cert`.
    #[garde(custom(requires(self.ssl_client_cert.as_ref(), "ssl_client_cert")))]
    pub ssl_client_key: Option<SecretRef>,
    /// Application name reported to the database server.
    #[garde(skip)]
    pub application_name: String,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_owned(),
            port: 5432,
            database: "magpie".to_owned(),
            username: "magpie".to_owned(),
            password: None,
            ssl_mode: SslMode::default(),
            ssl_root_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            application_name: "magpie".to_owned(),
        }
    }
}

/// TLS policy for the Postgres connection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SslMode {
    /// No TLS.
    Disable,
    /// TLS when the server offers it, without verification. Falls back to plain text.
    Prefer,
    /// TLS required. No certificate verification.
    Require,
    /// TLS required. The server certificate must chain to a trusted CA.
    VerifyCa,
    /// TLS required. The certificate must chain to a trusted CA and match the host name.
    #[default]
    VerifyFull,
}

impl From<SslMode> for PgSslMode {
    fn from(mode: SslMode) -> Self {
        match mode {
            SslMode::Disable => Self::Disable,
            SslMode::Prefer => Self::Prefer,
            SslMode::Require => Self::Require,
            SslMode::VerifyCa => Self::VerifyCa,
            SslMode::VerifyFull => Self::VerifyFull,
        }
    }
}

/// SQLite settings.
#[derive(Debug, Serialize, Deserialize, garde::Validate)]
#[serde(default, deny_unknown_fields)]
pub struct SqliteConfig {
    /// Database file path. Created when missing.
    #[garde(skip)]
    pub path: PathBuf,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_SQLITE_PATH),
        }
    }
}

/// Database flags. Each one overrides the matching config value when given.
#[derive(Debug, Clone, Args, Serialize)]
pub struct DbArgs {
    #[command(flatten)]
    pool: PoolArgs,
    // An empty table has no `type` and would fail, so it is omitted without flags.
    #[command(flatten)]
    #[serde(skip_serializing_if = "BackendArgs::is_empty")]
    backend: BackendArgs,
}

#[derive(Debug, Clone, Args, Serialize)]
#[command(next_help_heading = "Database")]
struct BackendArgs {
    /// Database backend.
    #[arg(
        id = "db-backend",
        long = "db-backend",
        value_name = "BACKEND",
        value_enum
    )]
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<BackendKind>,
    #[command(flatten)]
    #[serde(flatten)]
    postgres: PostgresArgs,
    #[command(flatten)]
    #[serde(flatten)]
    sqlite: SqliteArgs,
}

impl BackendArgs {
    const fn is_empty(&self) -> bool {
        self.kind.is_none() && self.postgres.is_empty() && self.sqlite.is_empty()
    }
}

/// Values of `--db-backend`, matching the `type` key.
#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum BackendKind {
    Postgres,
    Sqlite,
}

#[derive(Debug, Clone, Args, Serialize)]
#[command(next_help_heading = "Database")]
struct PoolArgs {
    /// Maximum database connections per instance.
    #[arg(
        id = "db-max-connections",
        long = "db-max-connections",
        value_name = "COUNT"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    max_connections: Option<u32>,
    /// Minimum database connections per instance.
    #[arg(
        id = "db-min-connections",
        long = "db-min-connections",
        value_name = "COUNT"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    min_connections: Option<u32>,
    /// Close idle database connections after this long, e.g. `5m`.
    #[arg(id = "db-idle-timeout", long = "db-idle-timeout", value_name = "DURATION", value_parser = magpie_core::duration::parse)]
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "magpie_core::duration::serialize_option"
    )]
    idle_timeout: Option<Duration>,
    /// Retire database connections at this age, e.g. `30m`.
    #[arg(id = "db-max-lifetime", long = "db-max-lifetime", value_name = "DURATION", value_parser = magpie_core::duration::parse)]
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "magpie_core::duration::serialize_option"
    )]
    max_lifetime: Option<Duration>,
}

#[derive(Debug, Clone, Args, Serialize)]
#[command(next_help_heading = "Database: Postgres")]
struct PostgresArgs {
    /// Postgres server host name, or a Unix socket directory.
    #[arg(id = "pg-host", long = "pg-host", value_name = "HOST")]
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    /// Postgres server port.
    #[arg(id = "pg-port", long = "pg-port", value_name = "PORT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    /// Postgres database name.
    #[arg(id = "pg-database", long = "pg-database", value_name = "NAME")]
    #[serde(skip_serializing_if = "Option::is_none")]
    database: Option<String>,
    /// Postgres role to connect as.
    #[arg(id = "pg-username", long = "pg-username", value_name = "ROLE")]
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    /// Postgres password as a reference, e.g. `env:PGPASSWORD` or `file:/run/secrets/pg`.
    #[arg(id = "pg-password", long = "pg-password", value_name = "REF", value_parser = SecretRefParser)]
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<SecretRef>,
    /// Postgres TLS policy.
    #[arg(
        id = "pg-ssl-mode",
        long = "pg-ssl-mode",
        value_name = "MODE",
        value_enum
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_mode: Option<SslMode>,
    /// CA certificate (PEM) used to verify the Postgres server, as a reference, e.g. `file:/etc/ssl/pg-ca.pem`.
    #[arg(
        id = "pg-ssl-root-cert",
        long = "pg-ssl-root-cert",
        value_name = "REF",
        value_parser = SecretRefParser
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_root_cert: Option<SecretRef>,
    /// Client certificate (PEM) for mutual TLS, as a reference, e.g. `file:/run/secrets/pg.crt`.
    #[arg(
        id = "pg-ssl-client-cert",
        long = "pg-ssl-client-cert",
        value_name = "REF",
        value_parser = SecretRefParser
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_client_cert: Option<SecretRef>,
    /// Client private key (PEM) for mutual TLS, as a reference, e.g. `file:/run/secrets/pg.key`.
    #[arg(id = "pg-ssl-client-key", long = "pg-ssl-client-key", value_name = "REF", value_parser = SecretRefParser)]
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_client_key: Option<SecretRef>,
}

impl PostgresArgs {
    const fn is_empty(&self) -> bool {
        self.host.is_none()
            && self.port.is_none()
            && self.database.is_none()
            && self.username.is_none()
            && self.password.is_none()
            && self.ssl_mode.is_none()
            && self.ssl_root_cert.is_none()
            && self.ssl_client_cert.is_none()
            && self.ssl_client_key.is_none()
    }
}

#[derive(Debug, Clone, Args, Serialize)]
#[command(next_help_heading = "Database: SQLite")]
struct SqliteArgs {
    /// Path to the SQLite database file.
    #[arg(id = "sqlite-path", long = "sqlite-path", value_name = "PATH")]
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
}

impl SqliteArgs {
    const fn is_empty(&self) -> bool {
        self.path.is_none()
    }
}
