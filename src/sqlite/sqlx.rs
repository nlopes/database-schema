//! Implementation of the `sqlite` feature for the `sqlx` backend

pub(crate) const DEFAULT_CONNECTION_URL: &str = "sqlite::memory:";

pub(crate) async fn fetch_structure_sql<P: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
) -> Result<String, sqlx::Error> {
    use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, ConnectOptions};
    use std::str::FromStr;

    let mut conn = SqliteConnectOptions::from_str(connection_url)?
        .connect()
        .await?;

    let migrator = Migrator::new(migrations_path.as_ref()).await?;
    migrator.run(&mut conn).await?;
    fetch_structure(&mut conn).await
}

async fn fetch_structure(conn: &mut sqlx::sqlite::SqliteConnection) -> Result<String, sqlx::Error> {
    use sqlx::Row;
    let structure_dump = sqlx::query(super::SQLITE_SCHEMA_QUERY)
        .fetch_all(conn)
        .await?
        .iter()
        .map(|row| {
            let name = row.get::<String, _>(0);
            let r#type = row.get::<String, _>(1);
            let sql = row.get::<String, _>(2);
            format!("--\n--  Name: {name}; Type: {type}\n--\n{sql};\n")
        })
        .collect::<Vec<String>>()
        .join("\n");
    Ok(structure_dump)
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "sqlx", feature = "sqlite"))]
    #[tokio::test]
    async fn test_fetch_structure_sql() -> Result<(), crate::error::Error> {
        let migrations_path = std::path::PathBuf::from("./fixtures/sqlx/sqlite/migrations");
        let structure =
            super::fetch_structure_sql(super::DEFAULT_CONNECTION_URL, migrations_path).await?;
        assert!(structure.contains("--\n--  Name: _sqlx_migrations; Type: table\n--\nCREATE TABLE _sqlx_migrations (\n    version BIGINT PRIMARY KEY,\n    description TEXT NOT NULL,\n    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,\n    success BOOLEAN NOT NULL,\n    checksum BLOB NOT NULL,\n    execution_time BIGINT NOT NULL\n);\n\n--\n--  Name: users; Type: table\n--\nCREATE TABLE users (\n  id TEXT PRIMARY KEY NOT NULL,\n  email TEXT NOT NULL,\n  created_at TEXT NOT NULL DEFAULT(datetime('now', 'utc'))\n);\n"));
        Ok(())
    }
}
