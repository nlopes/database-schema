//! Error types for the library.

/// Error types for the library.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    /// Any kind of IO error
    IOError(#[from] std::io::Error),
    #[error("Command run error: {0}")]
    /// Any kind of error when running a command
    CommandRunError(String),
    #[cfg(feature = "sqlx")]
    #[error("DB error: {0}")]
    /// Any kind of database engine related error
    DBError(#[from] sqlx::Error),
    #[cfg(feature = "diesel")]
    #[error("DB error: {0}")]
    /// Any kind of database engine related error
    DBError(#[from] diesel::result::Error),
    #[cfg(feature = "diesel")]
    #[error("DB error: {0}")]
    /// Any connection error when connecting to the database in `diesel`
    DBConnectionError(#[from] diesel::ConnectionError),
    #[cfg(feature = "diesel")]
    #[error("DB error: {0}")]
    /// Any connection error when running migrations in `diesel`
    MigrationError(#[from] diesel_migrations::MigrationError),
    #[cfg(any(feature = "mysql", feature = "postgres"))]
    #[error("Unable to extract database name from connection string")]
    /// Extracting the database name from the connection string failed
    ExtractDatabaseNameError,
    #[cfg(any(feature = "mysql", feature = "postgres"))]
    #[error("Uri configuration error: {0}")]
    /// Error when parsing the connection string
    UriConfiguration(String),
    #[cfg(any(feature = "mysql", feature = "postgres"))]
    #[error("Uri configuration encoding error: {0}")]
    /// Error when decoding parts of the connection string
    UriConfigurationDecoding(#[from] core::str::Utf8Error),
}
