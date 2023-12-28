#![doc = include_str!("../README.md")]
#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    unused_extern_crates,
    unused_results,
    variant_size_differences
)]
#![cfg_attr(docsrs, feature(doc_cfg), allow(unused_attributes))]

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql"),))]
compile_error!(
    "At least one of the following features must be enabled: sqlite, postgres or mysql."
);
#[cfg(not(any(feature = "sqlx", feature = "diesel"),))]
compile_error!("At least one of the following features must be enabled: sqlx or diesel.");

#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite;

#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
mod mysql;

#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
mod postgres;

#[cfg(all(
    feature = "macros",
    not(any(feature = "runtime-async-std", feature = "runtime-tokio"))
))]
compile_error!(
    "At least one of the following features must be enabled: runtime-async-std or runtime-tokio."
);

#[cfg(all(
    feature = "macros",
    any(feature = "runtime-async-std", feature = "runtime-tokio")
))]
pub mod macros;

pub(crate) mod process;

pub mod error;
pub use error::Error;

/// Entry point for using the crate and the result of calling [`DatabaseSchemaBuilder::build`].
///
/// ```rust,ignore
/// DatabaseSchemaBuilder::new().build().dump().await?;
/// ```
#[cfg(all(
    any(feature = "sqlite", feature = "postgres", feature = "mysql"),
    any(feature = "sqlx", feature = "diesel")
))]
#[derive(Debug, Default)]
pub struct DatabaseSchema(DatabaseSchemaInner);

#[derive(Debug, Clone)]
struct ConnectionUrl(String);

impl Default for ConnectionUrl {
    fn default() -> Self {
        #[cfg(feature = "sqlite")]
        let conn = ConnectionUrl(String::from(sqlite::DEFAULT_CONNECTION_URL));
        #[cfg(feature = "mysql")]
        let conn = ConnectionUrl(String::from(mysql::DEFAULT_CONNECTION_URL));
        #[cfg(feature = "postgres")]
        let conn = ConnectionUrl(String::from(postgres::DEFAULT_CONNECTION_URL));

        conn
    }
}

#[derive(Debug, Default, Clone)]
struct DatabaseSchemaInner {
    connection_url: ConnectionUrl,
    migrations_path: std::path::PathBuf,
    destination_path: std::path::PathBuf,
}

/// Builder for `DatabaseSchema`
#[cfg(all(
    any(feature = "sqlite", feature = "postgres", feature = "mysql"),
    any(feature = "sqlx", feature = "diesel")
))]
#[derive(Debug, Default)]
pub struct DatabaseSchemaBuilder(DatabaseSchemaInner);

#[cfg(all(
    any(feature = "sqlite", feature = "postgres", feature = "mysql"),
    any(feature = "sqlx", feature = "diesel")
))]
#[allow(dead_code)]
impl DatabaseSchemaBuilder {
    /// Create a new `DatabaseSchemaBuilder`
    pub fn new() -> Self {
        Self::default()
    }

    /// This is the connection URL used to connect to the database.
    ///
    /// For `mysql` and `postgres` this is the same URL you would pass to the `connect` method of the client.
    ///
    /// * `mysql`: `mysql://[user[:password]@]host/database_name[?unix_socket=socket-path&ssl_mode=SSL_MODE*&ssl_ca=/etc/ssl/certs/ca-certificates.crt&ssl_cert=/etc/ssl/certs/client-cert.crt&ssl_key=/etc/ssl/certs/client-key.crt]`
    ///
    /// * `postgres`: `postgresql://[user[:password]@][netloc][:port][/dbname][?param1=value1&...]` - you can read more at [libpq docs](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING)
    ///
    /// * `sqlite`: `sqlite::memory:` in the case of `sqlx` and `:memory:` in the case of
    /// `diesel` - you don't need to set this for `sqlite` as we auto-detect it as long as
    /// you enable the `sqlite` feature.
    pub fn connection_url<S: Into<String>>(&mut self, connection_url: S) -> &mut Self {
        self.0.connection_url = ConnectionUrl(connection_url.into());
        self
    }

    /// Set `migrations_dir` - this is the directory path where your migrations are stored.
    ///
    /// By default we assume that the migrations are stored in the `migrations` directory
    /// starting at the root of your project.
    ///
    /// We call `canonicalize()` on the path, so you can pass in a relative path. The
    /// downside is that this call can fail.
    pub fn migrations_dir<P: AsRef<std::path::Path>>(
        &mut self,
        migrations_dir: P,
    ) -> Result<&mut Self, Error> {
        self.0.migrations_path = migrations_dir.as_ref().to_path_buf().canonicalize()?;
        Ok(self)
    }

    /// Set `destination_path` - this is the path to the file where we'll store the SQL dump.
    ///
    /// By default we assume `structure.sql` in the root of your project.
    pub fn destination_path<P: AsRef<std::path::Path>>(
        &mut self,
        destination_path: P,
    ) -> &mut Self {
        self.0.destination_path = destination_path.as_ref().to_path_buf();
        self
    }

    /// Build `DatabaseSchema` from `DatabaseSchemaBuilder`
    pub fn build(&self) -> DatabaseSchema {
        DatabaseSchema(self.0.clone())
    }
}

#[cfg(all(
    any(feature = "sqlite", feature = "postgres", feature = "mysql"),
    any(feature = "sqlx", feature = "diesel")
))]
impl DatabaseSchema {
    /// Dump the database schema.
    pub async fn dump(&self) -> Result<(), Error> {
        #[cfg(all(feature = "mysql", any(feature = "sqlx", feature = "diesel")))]
        use crate::mysql::write_structure_sql;
        #[cfg(all(feature = "postgres", any(feature = "sqlx", feature = "diesel")))]
        use crate::postgres::write_structure_sql;
        #[cfg(all(feature = "sqlite", any(feature = "sqlx", feature = "diesel")))]
        use crate::sqlite::write_structure_sql;

        write_structure_sql(
            &self.0.connection_url.0,
            self.0.migrations_path.clone(),
            self.0.destination_path.clone(),
        )
        .await?;
        Ok(())
    }
}

/// Generate a `destination_path` SQL file using migrations from the `migrations_path`
/// folder.
///
/// Calling this function is strictly equivalent to:
///
/// ```rust,ignore
/// // This assumes you're using SQLite in memory.
/// //
/// // If you need to set up a `connection_url` you can use `DatabaseSchemaBuilder::connection_url`
/// // before calling `build()`.
///
/// DatabaseSchemaBuilder::new()
///     .migrations_dir(migrations_path)?
///     .destination_path(destination_path)
///     .build()
///     .dump()
///     .await
/// ```
/// Requires an executor to be available.
#[cfg(all(
    any(feature = "sqlite", feature = "postgres", feature = "mysql"),
    any(feature = "sqlx", feature = "diesel")
))]
pub async fn generate<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    connection_url: Option<&str>,
    migrations_path: P,
    destination_path: Q,
) -> Result<(), Error> {
    let mut builder = DatabaseSchemaBuilder::new();
    if let Some(connection_url) = connection_url {
        let _ = builder.connection_url(connection_url);
    }
    builder
        .migrations_dir(migrations_path)?
        .destination_path(destination_path)
        .build()
        .dump()
        .await
}
