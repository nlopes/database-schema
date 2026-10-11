#![cfg(feature = "sqlite")]

use database_schema::{generate, DatabaseSchema, DatabaseSchemaBuilder};
use std::path::{Path, PathBuf};

fn migrations(root: &Path, sql: &str) -> std::io::Result<PathBuf> {
    let path = root.join("migrations");
    std::fs::create_dir_all(&path)?;
    #[cfg(feature = "diesel")]
    {
        let migration = path.join("2026-10-10-000001_schema");
        std::fs::create_dir_all(&migration)?;
        std::fs::write(migration.join("up.sql"), sql)?;
        std::fs::write(migration.join("down.sql"), "SELECT 1;")?;
    }
    #[cfg(feature = "sqlx")]
    std::fs::write(path.join("1_schema.sql"), sql)?;
    Ok(path)
}

#[tokio::test]
async fn dump_restores_tables_views_indexes_and_triggers() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let migrations = migrations(
        root.path(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, email TEXT);
         CREATE TABLE sqlitex_users (id INTEGER);
         CREATE TABLE audit (id INTEGER);
         CREATE VIEW user_emails AS SELECT email FROM users;
         CREATE INDEX user_email ON users (email);
         CREATE TRIGGER user_insert AFTER INSERT ON users BEGIN
           INSERT INTO audit (id) VALUES (NEW.id);
         END;",
    )?;
    let output = root.path().join("structure.sql");
    generate(None, migrations, &output).await?;
    let dump = std::fs::read_to_string(output)?;
    assert!(dump.contains("CREATE TABLE sqlitex_users"));
    assert!(!dump.contains("CREATE TABLE sqlite_sequence"));

    #[cfg(feature = "diesel")]
    {
        use diesel::{connection::SimpleConnection, Connection, RunQueryDsl};
        let mut restored = diesel::SqliteConnection::establish(":memory:")?;
        restored.batch_execute(&dump)?;
        restored.batch_execute("INSERT INTO users (email) VALUES ('test@example.org');")?;
        restored.batch_execute("SELECT email FROM user_emails; DROP INDEX user_email;")?;
        assert_eq!(
            diesel::sql_query("UPDATE audit SET id = id").execute(&mut restored)?,
            1
        );
        assert_eq!(
            diesel::sql_query("UPDATE sqlitex_users SET id = id").execute(&mut restored)?,
            0
        );
    }
    #[cfg(feature = "sqlx")]
    {
        use sqlx::{Connection, Executor};
        let mut restored = sqlx::SqliteConnection::connect("sqlite::memory:").await?;
        let _ = restored
            .execute(sqlx::raw_sql(sqlx::AssertSqlSafe(dump.as_str())))
            .await?;
        let _ = restored
            .execute("INSERT INTO users (email) VALUES ('test@example.org');")
            .await?;
        let _ = restored.execute("SELECT email FROM user_emails;").await?;
        let _ = restored.execute("DROP INDEX user_email;").await?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit")
            .fetch_one(&mut restored)
            .await?;
        assert_eq!(count, 1);
        let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlitex_users")
            .fetch_one(&mut restored)
            .await?;
    }
    Ok(())
}

#[tokio::test]
async fn failed_migration_preserves_destination() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let migrations = migrations(root.path(), "THIS IS INVALID SQL;")?;
    let output = root.path().join("structure.sql");
    std::fs::write(&output, "existing schema")?;
    assert!(generate(None, migrations, &output).await.is_err());
    assert_eq!(std::fs::read_to_string(output)?, "existing schema");
    Ok(())
}

#[test]
fn documented_defaults_generate_structure() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let _ = migrations(root.path(), "CREATE TABLE default_users (id INTEGER);")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "defaults_child", "--nocapture"])
        .env("DATABASE_SCHEMA_TEST_DEFAULTS", "1")
        .current_dir(root.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::fs::read_to_string(root.path().join("structure.sql"))?
        .contains("CREATE TABLE default_users"));
    Ok(())
}

#[test]
fn defaults_child() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("DATABASE_SCHEMA_TEST_DEFAULTS").is_none() {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    for schema in [
        DatabaseSchema::default(),
        DatabaseSchemaBuilder::default().build(),
        DatabaseSchemaBuilder::new().build(),
    ] {
        runtime.block_on(schema.dump())?;
        assert!(std::fs::read_to_string("structure.sql")?.contains("CREATE TABLE default_users"));
    }
    #[cfg(feature = "macros")]
    {
        std::fs::remove_file("structure.sql")?;
        database_schema::generate_without_runtime_using_defaults!();
        assert!(std::fs::read_to_string("structure.sql")?.contains("CREATE TABLE default_users"));
    }
    Ok(())
}

#[cfg(feature = "macros")]
#[test]
fn macro_works_in_expression_position() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let migrations = migrations(root.path(), "CREATE TABLE macro_users (id INTEGER);")?;
    let output = root.path().join("structure.sql");
    let _: () = database_schema::generate_without_runtime!(migrations, &output);
    assert!(std::fs::read_to_string(output)?.contains("CREATE TABLE macro_users"));
    Ok(())
}
