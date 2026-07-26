//! Implementation of the `sqlite` feature for the `diesel` backend

pub(crate) const DEFAULT_CONNECTION_URL: &str = ":memory:";

pub(crate) async fn fetch_structure_sql<P: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
) -> Result<String, crate::error::Error> {
    use diesel::Connection;
    use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationHarness};
    let mut conn = diesel::SqliteConnection::establish(connection_url)?;
    let migrations = FileBasedMigrations::from_path(migrations_path)?;
    let _ = HarnessWithOutput::write_to_stdout(&mut conn)
        .run_pending_migrations(migrations)
        .map(|_| ());

    Ok(fetch_structure(&mut conn).await?)
}

pub(crate) async fn fetch_structure(
    conn: &mut diesel::SqliteConnection,
) -> Result<String, diesel::result::Error> {
    use diesel::{sql_query, QueryableByName, RunQueryDsl};

    #[allow(unused_qualifications)]
    mod schema {
        diesel::table! {
            sqlite_schema(name) {
                name -> Text,
                mytype -> Text,
                sql -> Text,
            }
        }
    }

    use schema::sqlite_schema;

    #[derive(Debug, QueryableByName)]
    #[diesel(table_name = sqlite_schema)]
    struct SqliteSchema {
        name: String,
        mytype: String,
        sql: String,
    }

    let results: Vec<SqliteSchema> = sql_query(super::SQLITE_SCHEMA_QUERY).load(conn)?;
    Ok(results
        .iter()
        .map(|r| {
            format!(
                "--\n--  Name: {}; Type: {}\n--\n{};\n",
                r.name, r.mytype, r.sql
            )
        })
        .collect::<Vec<String>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "diesel", feature = "sqlite"))]
    #[tokio::test]
    async fn test_fetch_structure_sql() -> Result<(), crate::error::Error> {
        let migrations_path = std::path::PathBuf::from("./fixtures/diesel/sqlite/migrations");
        let structure =
            super::fetch_structure_sql(super::DEFAULT_CONNECTION_URL, migrations_path).await?;

        assert!(structure.contains("--\n--  Name: __diesel_schema_migrations; Type: table\n--\nCREATE TABLE __diesel_schema_migrations (\n       version VARCHAR(50) PRIMARY KEY NOT NULL,\n       run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP\n);\n\n--\n--  Name: users; Type: table\n--\nCREATE TABLE users (\n  id TEXT PRIMARY KEY NOT NULL,\n  email TEXT NOT NULL,\n  created_at TEXT NOT NULL DEFAULT(datetime('now', 'utc'))\n)"));
        Ok(())
    }
}
