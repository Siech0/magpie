use std::path::PathBuf;

use clap::{Args, ValueEnum};
use magpie_core::secret::{SecretRef, SecretRefParser};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(debug_assertions)]
const DEFAULT_SQLITE_PATH: &str = "magpie.db";
#[cfg(not(debug_assertions))]
const DEFAULT_SQLITE_PATH: &str = "/var/lib/magpie/magpie.db";

/// Database settings. Exactly one backend, SQLite when none is configured.
#[derive(Debug, Default)]
pub struct DbConfig {
    /// The selected backend.
    pub backend: Backend,
}

/// The database backend.
#[derive(Debug)]
pub enum Backend {
    /// A Postgres server.
    Postgres(PostgresConfig),
    /// A local SQLite file.
    Sqlite(SqliteConfig),
}

impl Default for Backend {
    fn default() -> Self {
        Self::Sqlite(SqliteConfig::default())
    }
}

/// The on-disk shape: two optional sections. Only exists at the serde boundary.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawDbConfig {
    postgres: Option<PostgresConfig>,
    sqlite: Option<SqliteConfig>,
}

impl<'de> Deserialize<'de> for DbConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDbConfig::deserialize(deserializer)?;
        let backend = match (raw.postgres, raw.sqlite) {
            (Some(postgres), None) => Backend::Postgres(postgres),
            (None, Some(sqlite)) => Backend::Sqlite(sqlite),
            (None, None) => Backend::default(),
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "set [db.postgres] or [db.sqlite], not both",
                ));
            }
        };
        Ok(Self { backend })
    }
}

/// Borrowed view for serializing
#[derive(Serialize)]
struct RawDbConfigRef<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    postgres: Option<&'a PostgresConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sqlite: Option<&'a SqliteConfig>,
}

impl Serialize for DbConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (postgres, sqlite) = match &self.backend {
            Backend::Postgres(postgres) => (Some(postgres), None),
            Backend::Sqlite(sqlite) => (None, Some(sqlite)),
        };
        RawDbConfigRef { postgres, sqlite }.serialize(serializer)
    }
}

/// Postgres connection settings.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PostgresConfig {
    /// Server host name, or a Unix socket directory.
    pub host: String,
    /// Server port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Role to connect as.
    pub username: String,
    /// Password, if the server uses password authentication.
    pub password: Option<SecretRef>,
    /// TLS policy.
    pub ssl_mode: SslMode,
    /// CA certificate (PEM) used to verify the server, instead of the default roots.
    pub ssl_root_cert: Option<PathBuf>,
    /// Client certificate (PEM) for mutual TLS. Requires `ssl_client_key`.
    pub ssl_client_cert: Option<PathBuf>,
    /// Client private key (PEM) for mutual TLS. Requires `ssl_client_cert`.
    pub ssl_client_key: Option<SecretRef>,
    /// Application name reported to the database server.
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
    /// TLS required, without certificate verification.
    Require,
    /// TLS required; the server certificate must chain to a trusted CA.
    VerifyCa,
    /// TLS required; the certificate must chain to a trusted CA and match the host name.
    #[default]
    VerifyFull,
}

/// SQLite settings.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SqliteConfig {
    /// Database file path. Created when missing.
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
    // An empty backend table would deserialize as a configured backend, so each
    // section is omitted entirely when none of its flags were given.
    #[command(flatten)]
    #[serde(skip_serializing_if = "PostgresArgs::is_empty")]
    postgres: PostgresArgs,
    #[command(flatten)]
    #[serde(skip_serializing_if = "SqliteArgs::is_empty")]
    sqlite: SqliteArgs,
}

#[derive(Debug, Clone, Args, Serialize)]
#[command(next_help_heading = "Database: Postgres (any flag selects Postgres)")]
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
    /// CA certificate (PEM) used to verify the Postgres server.
    #[arg(
        id = "pg-ssl-root-cert",
        long = "pg-ssl-root-cert",
        value_name = "PATH"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_root_cert: Option<PathBuf>,
    /// Client certificate (PEM) for mutual TLS.
    #[arg(
        id = "pg-ssl-client-cert",
        long = "pg-ssl-client-cert",
        value_name = "PATH"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_client_cert: Option<PathBuf>,
    /// Client private key (PEM) for mutual TLS, as a reference, e.g. `file:/run/secrets/pg.key`.
    ///
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
#[command(next_help_heading = "Database: SQLite (any flag selects SQLite)")]
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
