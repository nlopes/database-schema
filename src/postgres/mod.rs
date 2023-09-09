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

#[cfg(test)]
mod tests {
    async fn call_write_structure_sql<M: AsRef<str>, D: AsRef<str>>(
        fixtures_path: M,
        destination_filename: D,
    ) -> Result<(), crate::error::Error> {
        let destination_path = std::env::temp_dir().join(destination_filename.as_ref());
        let migrations_path = std::path::PathBuf::from(fixtures_path.as_ref()).join("migrations");
        let _ = super::write_structure_sql(
            super::DEFAULT_CONNECTION_URL,
            migrations_path,
            &destination_path,
        )
        .await?;
        let expected = std::fs::read_to_string(dbg!(format!(
            "./{}/{}",
            fixtures_path.as_ref(),
            destination_filename.as_ref()
        )))?;
        let contents = std::fs::read_to_string(destination_path)?;
        assert_eq!(contents, expected);
        Ok(())
    }

    #[cfg(all(feature = "sqlx", feature = "postgres"))]
    #[tokio::test]
    async fn test_write_structure_sql() -> Result<(), crate::error::Error> {
        call_write_structure_sql("./fixtures/sqlx/postgres", "sqlx-postgres-structure.sql").await
    }

    #[cfg(all(feature = "diesel", feature = "postgres"))]
    #[tokio::test]
    async fn test_write_structure_sql() -> Result<(), crate::error::Error> {
        call_write_structure_sql(
            "./fixtures/diesel/postgres",
            "diesel-postgres-structure.sql",
        )
        .await
    }
}
