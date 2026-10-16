use magpie_core::secret::SecretError;

/// Errors from connecting to the database.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// A secret in the database config could not be resolved.
    #[error("could not resolve `{field}`")]
    Secret {
        /// The config key, e.g. `db.backend.password`.
        field: &'static str,
        /// Why resolving failed.
        #[source]
        source: SecretError,
    },

    /// The pool could not open its connections.
    #[error("could not connect to the database")]
    Connect(#[from] sqlx::Error),

    /// A migration failed or the applied history does not match the embedded migrations.
    #[error("could not migrate the database")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}
