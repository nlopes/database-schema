//! Implementation of the `postgres` feature

pub(crate) const DEFAULT_CONNECTION_URL: &str = "postgresql://root:@localhost:5432/postgres";

use crate::error::Error;

#[allow(unused_results)]
#[cfg(any(feature = "sqlx", feature = "diesel"))]
pub(crate) async fn write_structure_sql<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
    destination_path: Q,
) -> Result<(), Error> {
    migrate(connection_url, migrations_path).await?;

    let mut cmd = std::process::Command::new("pg_dump");
    cmd.arg("--schema-only")
        .arg("--no-owner")
        .arg("--no-privileges")
        .arg("--file")
        .arg(destination_path.as_ref())
        .arg(connection_url);
    crate::process::run(&mut cmd).await?;
    Ok(())
}

#[cfg(feature = "sqlx")]
async fn migrate<P: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
) -> Result<(), sqlx::Error> {
    use sqlx::{
        migrate::{Migrate, Migrator},
        postgres::PgConnectOptions,
        ConnectOptions,
    };
    use std::str::FromStr;

    let mut conn = PgConnectOptions::from_str(connection_url)?
        .connect()
        .await?;

    // Ensure the migrations table exists before we run the migrations
    conn.ensure_migrations_table().await?;

    let migrator = Migrator::new(migrations_path.as_ref()).await?;
    migrator.run_direct(&mut conn).await?;
    Ok(())
}

#[cfg(feature = "diesel")]
async fn migrate<P: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
) -> Result<(), Error> {
    use diesel::Connection;
    use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationHarness};
    let mut conn = diesel::PgConnection::establish(connection_url)?;
    let migrations = FileBasedMigrations::from_path(migrations_path)?;
    let _ = HarnessWithOutput::write_to_stdout(&mut conn)
        .run_pending_migrations(migrations)
        .map(|_| ());
    Ok(())
}
