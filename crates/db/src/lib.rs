//! Database access and migrations.
mod config;
mod error;

pub use config::{Backend, DbArgs, DbConfig, PostgresConfig, SqliteConfig, SslMode};
pub use error::DbError;

use config::PoolConfig;
use magpie_core::secret::{Secret, SecretRef};
use sqlx::{
    Connection as _, Database,
    migrate::Migrator,
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgPool},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous},
};

static POSTGRES_MIGRATIONS: Migrator = sqlx::migrate!("./migrations/postgres");
static SQLITE_MIGRATIONS: Migrator = sqlx::migrate!("./migrations/sqlite");

/// Database handle.
#[derive(Debug, Clone)]
pub struct Db {
    pool: Pool,
}

#[derive(Debug, Clone)]
enum Pool {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

/// Runs `$body` with `$pool` bound to the backend's pool.
macro_rules! on_pool {
    ($db:expr, $pool:ident => $body:expr) => {
        match &$db.pool {
            Pool::Postgres($pool) => $body,
            Pool::Sqlite($pool) => $body,
        }
    };
}

impl Db {
    /// Opens a pool for the configured backend.
    ///
    /// # Errors
    ///
    /// Returns [`DbError`] when a secret cannot be resolved or the database cannot be reached.
    pub async fn connect(config: &DbConfig) -> Result<Self, DbError> {
        match &config.backend {
            Backend::Postgres(backend) => {
                let mut connect_options = PgConnectOptions::new()
                    .host(&backend.host)
                    .port(backend.port)
                    .database(&backend.database)
                    .username(&backend.username)
                    .ssl_mode(backend.ssl_mode.into())
                    .application_name(&backend.application_name);

                if let Some(password) = &backend.password {
                    let password = resolve(password, "db.backend.password")?;
                    connect_options = connect_options.password(password.expose());
                }

                if let Some(pem) = &backend.ssl_root_cert {
                    let pem = resolve(pem, "db.backend.ssl_root_cert")?;
                    connect_options =
                        connect_options.ssl_root_cert_from_pem(pem.expose().as_bytes().to_vec());
                }

                if let Some(pem) = &backend.ssl_client_cert {
                    let pem = resolve(pem, "db.backend.ssl_client_cert")?;
                    connect_options = connect_options.ssl_client_cert_from_pem(pem.expose());
                }

                if let Some(pem) = &backend.ssl_client_key {
                    let pem = resolve(pem, "db.backend.ssl_client_key")?;
                    connect_options = connect_options.ssl_client_key_from_pem(pem.expose());
                }

                let pool = pool_options(&config.pool)
                    .connect_with(connect_options)
                    .await?;
                Ok(Self {
                    pool: Pool::Postgres(pool),
                })
            }
            Backend::Sqlite(backend) => {
                if !backend.path.is_file() {
                    let path =
                        std::path::absolute(&backend.path).unwrap_or_else(|_| backend.path.clone());
                    tracing::warn!(
                        "no database file found at {}, database will be created",
                        path.display()
                    );
                }

                let connect_options = SqliteConnectOptions::new()
                    .filename(&backend.path)
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal)
                    .foreign_keys(true)
                    .busy_timeout(backend.busy_timeout);

                let pool = pool_options(&config.pool)
                    .connect_with(connect_options)
                    .await?;
                Ok(Self {
                    pool: Pool::Sqlite(pool),
                })
            }
        }
    }

    /// Applies pending migrations for the backend.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Migrate`] when a migration fails or an applied one has changed.
    pub async fn migrate(&self) -> Result<(), DbError> {
        match &self.pool {
            Pool::Postgres(pool) => POSTGRES_MIGRATIONS.run(pool).await?,
            Pool::Sqlite(pool) => SQLITE_MIGRATIONS.run(pool).await?,
        }
        Ok(())
    }

    /// Checks that the database answers a query.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Connect`] when no connection is available or the query fails.
    pub async fn ping(&self) -> Result<(), DbError> {
        on_pool!(self, pool => pool.acquire().await?.ping().await)?;
        Ok(())
    }
}

fn pool_options<DB: Database>(pool: &PoolConfig) -> PoolOptions<DB> {
    PoolOptions::new()
        .max_connections(pool.max_connections)
        .min_connections(pool.min_connections)
        .acquire_timeout(pool.acquire_timeout)
        .idle_timeout(pool.idle_timeout)
        .max_lifetime(pool.max_lifetime)
}

fn resolve(secret: &SecretRef, field: &'static str) -> Result<Secret, DbError> {
    secret
        .resolve()
        .map_err(|source| DbError::Secret { field, source })
}
