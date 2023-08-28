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
#![cfg_attr(docsrs, doc = include_str!("../README.md"), allow(unused_attributes))]

//! This crate provides a simple way to dump a database structure to a file, in SQL
//! format.
//!
//! It takes inspiration by the ruby on rails [schema dump].
//!
//! # Usage
//!
//! ```rust,ignore
//! use std::path::PathBuf;
//!
//! database_schema::generate_without_runtime_using_defaults!();
//! ```
//!
//!
//! # Feature flags
//!
//! `database-schema` uses a set of [feature flags] to reduce the size of the libray and
//! therefore your binary. The way one should use this package is to pick the right
//! combination of feature flags for their use case. Below is a list of the available
//! feature flags and the combinations that are recommended for each use case.
//!
//! - `sqlite`: Enables SQLite support.
//! - `postgres`: Enables PostgreSQL support.
//! - `mysql`: Enables MySQL support.
//! - `sqlx`: Enables [sqlx] support.
//! - `diesel`: Enables [diesel] support.
//!
//! ## Feature flag matrix
//! | Database | Query builder | Runtime |
//! |----------|---------------|---------|
//! | `sqlite` | `sqlx`        | `runtime-async-std` |
//! | `sqlite` | `sqlx`        | `runtime-tokio` |
//! | `sqlite` | `diesel`      | |
//! | `mysql`  | `sqlx`        | `runtime-async-std` |
//! | `mysql`  | `sqlx`        | `runtime-tokio` |
//! | `mysql`  | `diesel`      | |
//! | `postgres` | `sqlx`      | `runtime-async-std` |
//! | `postgres` | `sqlx`      | `runtime-tokio` |
//! | `postgres` | `diesel`    | |
//!
//! ## Combining feature flags
//!
//! The following are the recommended feature flag combinations for each use case.
//!
//! First pick one of the following database feature flags:
//!
//! * `sqlite`
//! * `mysql`
//! * `postgres`
//!
//! Then pick one of the following database query building feature flags:
//!
//! * `sqlx`
//! * `diesel`
//!
//! If you're using `sqlx`, you also have to pick one of the following runtime feature flags:
//!
//! * `runtime-async-std`
//! * `runtime-tokio`
//!
//! ## Example
//!
//! ```toml
//! [dependencies]
//! database-schema = { version = "0.1", features = ["sqlite", "sqlx", "runtime-async-std"] }
//! ```
//!
//! alternatively, if you're using `diesel`:
//! ```toml
//! [dependencies]
//! database-schema = { version = "0.1", features = ["sqlite", "diesel"] }
//! ```
//!
//! ## Macros
//!
//! This crate also provides a set of macros that can be used to generate the SQL
//! structure of a database at compile time. This is useful for generating the SQL from
//! `build.rs`.
//!
//!
//! ```toml
//! [dependencies]
//! database-schema = { version = "0.1", features = ["sqlite", "diesel", "macros"] }
//! ```
//!
//! ```rust,ignore
//! use database_schema::macros::generate_without_runtime;
//!
//! let sql = generate_without_runtime!("./migrations", "structure.sql");
//! ```
//!
//! The above is strictly equivalent to calling:
//!
//! ```rust,ignore
//! use database_schema::macros::generate_without_runtime_using_defaults;
//!
//! let sql = generate_without_runtime!();
//! ```
//!
//! # Customization
//!
//! ```rust,ignore
//! use database_schema::DatabaseSchemaBuilder;
//!
//! let migrations_path = "db/migrations";
//! let destination_path = "db/structure.sql";
//!
//! // This assumes you're using SQLite in memory.
//! //
//! // If you need to set up a `connection_url` you can use
//! // `DatabaseSchemaBuilder::connection_url` before calling
//! // `build()`.
//!
//! DatabaseSchemaBuilder::new()
//!     .migrations_dir(migrations_path)?
//!     .destination_path(destination_path)
//!     .build()
//!     .dump()
//!     .await
//! ```
//!
//! [feature flags]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section
//! [sqlx]: https://docs.rs/sqlx/latest/sqlx/
//! [diesel]: https://docs.rs/diesel/latest/diesel/
//! [schema dump]: https://guides.rubyonrails.org/active_record_migrations.html#schema-dumping-and-you

#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql"),))]
compile_error!(
    "At least one of the following features must be enabled: sqlite, postgres or mysql."
);
#[cfg(not(any(feature = "sqlx", feature = "diesel"),))]
compile_error!("At least one of the following features must be enabled: sqlx or diesel.");

#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
mod sqlite;

#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
mod mysql;

//#[cfg(feature = "postgres")]
//mod postgres;

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
        //#[cfg(feature = "postgres")]
        //let conn = ConnectionUrl(String::from(postgres::DEFAULT_CONNECTION_URL));

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

impl DatabaseSchema {
    /// Dump the database schema.
    pub async fn dump(&self) -> Result<(), Error> {
        #[cfg(all(feature = "mysql", any(feature = "sqlx", feature = "diesel")))]
        use crate::mysql::write_structure_sql;
        #[cfg(all(feature = "sqlite", any(feature = "sqlx", feature = "diesel")))]
        use crate::sqlite::write_structure_sql;
        //#[cfg(all(feature = "postgres", any(feature = "sqlx", feature = "diesel")))]
        //use crate::postgres::write_structure_sql;

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
