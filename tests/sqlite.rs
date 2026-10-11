#![cfg(feature = "sqlite")]

use database_schema::generate;
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
async fn failed_migration_preserves_destination() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let migrations = migrations(root.path(), "THIS IS INVALID SQL;")?;
    let output = root.path().join("structure.sql");
    std::fs::write(&output, "existing schema")?;
    assert!(generate(None, migrations, &output).await.is_err());
    assert_eq!(std::fs::read_to_string(output)?, "existing schema");
    Ok(())
}
