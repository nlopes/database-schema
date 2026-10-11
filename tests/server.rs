#![cfg(any(feature = "mysql", feature = "postgres"))]

use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(feature = "mysql")]
const ADMIN_URL: &str = "mysql://root:@127.0.0.1:3306/mysql";
#[cfg(feature = "postgres")]
const ADMIN_URL: &str = "postgresql://root:@127.0.0.1:5432/postgres";

async fn admin(sql: &str) -> TestResult {
    #[cfg(feature = "sqlx")]
    {
        use sqlx::{Connection, Executor};
        #[cfg(feature = "mysql")]
        let mut conn = sqlx::MySqlConnection::connect(ADMIN_URL).await?;
        #[cfg(feature = "postgres")]
        let mut conn = sqlx::PgConnection::connect(ADMIN_URL).await?;
        let _ = conn
            .execute(sqlx::raw_sql(sqlx::AssertSqlSafe(sql)))
            .await?;
    }
    #[cfg(feature = "diesel")]
    {
        use diesel::{connection::SimpleConnection, Connection};
        #[cfg(feature = "mysql")]
        let mut conn = diesel::MysqlConnection::establish(ADMIN_URL)?;
        #[cfg(feature = "postgres")]
        let mut conn = diesel::PgConnection::establish(ADMIN_URL)?;
        conn.batch_execute(sql)?;
    }
    Ok(())
}

fn migration(path: &Path, version: u8, sql: &str) -> std::io::Result<()> {
    #[cfg(feature = "diesel")]
    {
        let path = path.join(format!("2026-10-10-00000{version}_schema"));
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join("up.sql"), sql)?;
        std::fs::write(path.join("down.sql"), "SELECT 1;")?;
    }
    #[cfg(feature = "sqlx")]
    std::fs::write(path.join(format!("{version}_schema.sql")), sql)?;
    Ok(())
}

#[tokio::test]
async fn authenticated_dump_and_failed_migration() -> TestResult {
    let root = tempfile::tempdir()?;
    let suffix = root
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    let database = format!("schema_test_{suffix}");
    #[cfg(feature = "mysql")]
    let password = "schema secret:'\"\\+@?=\t";
    #[cfg(feature = "postgres")]
    let password = "schema secret:'\"\\+@?=";
    let escaped = password.replace('\\', "\\\\").replace('\'', "''");
    #[cfg(feature = "mysql")]
    {
        admin(&format!("CREATE DATABASE {database}")).await?;
        admin(&format!(
            "CREATE USER '{database}'@'%' IDENTIFIED BY '{escaped}'"
        ))
        .await?;
        admin(&format!("GRANT ALL PRIVILEGES ON *.* TO '{database}'@'%'")).await?;
    }
    #[cfg(feature = "postgres")]
    {
        admin(&format!(
            "CREATE ROLE {database} LOGIN PASSWORD E'{escaped}'"
        ))
        .await?;
        admin(&format!("CREATE DATABASE {database} OWNER {database}")).await?;
    }
    let result: TestResult = async {
        let migrations = root.path().join("migrations");
        std::fs::create_dir_all(&migrations)?;
        migration(
            &migrations,
            1,
            "CREATE TABLE schema_users (id INTEGER PRIMARY KEY);",
        )?;
        let output = root.path().join("structure.sql");
        #[cfg(feature = "mysql")]
        let prefix = "mysql";
        #[cfg(feature = "postgres")]
        let prefix = "postgresql";
        #[cfg(feature = "mysql")]
        let port = 3306;
        #[cfg(feature = "postgres")]
        let port = 5432;
        let encoded =
            percent_encoding::utf8_percent_encode(password, percent_encoding::NON_ALPHANUMERIC);
        let url = format!("{prefix}://{database}:{encoded}@127.0.0.1:{port}/{database}");
        let wrong_url = format!("{prefix}://{database}:wrong_password@127.0.0.1:{port}/{database}");
        assert!(
            database_schema::generate(Some(&wrong_url), &migrations, &output)
                .await
                .is_err(),
            "test server must require this account's password"
        );
        database_schema::generate(Some(&url), &migrations, &output).await?;
        #[cfg(feature = "postgres")]
        {
            let query_url = format!("postgresql://{database}@127.0.0.1:{port}/{database}?password={encoded}&application_name=schema_test");
            database_schema::generate(Some(&query_url), &migrations, &output).await?;
            #[cfg(feature = "diesel")]
            {
                let keyword_password = password.replace('\\', "\\\\").replace('\'', "\\'");
                let keyword_url = format!("host=127.0.0.1 port={port} user={database} dbname={database} password='{keyword_password}' application_name=schema_test");
                database_schema::generate(Some(&keyword_url), &migrations, &output).await?;
            }
            let service_file = root.path().join("pg_service.conf");
            std::fs::write(&service_file, "[schema_test]\npassword=wrong_service_password\n")?;
            let service_output = root.path().join("service.sql");
            let child = std::process::Command::new(std::env::current_exe()?)
                .args(["--exact", "postgres_service_child", "--nocapture"])
                .env("PGSERVICEFILE", &service_file)
                .env("DATABASE_SCHEMA_TEST_URL", format!("{url}?service=schema_test"))
                .env("DATABASE_SCHEMA_TEST_MIGRATIONS", &migrations)
                .env("DATABASE_SCHEMA_TEST_OUTPUT", &service_output)
                .output()?;
            assert!(child.status.success(), "{}{}", String::from_utf8_lossy(&child.stdout), String::from_utf8_lossy(&child.stderr));
            assert!(std::fs::read_to_string(service_output)?.contains("schema_users"));
        }
        let dump = std::fs::read_to_string(&output)?;
        assert!(dump.contains("schema_users"));
        migration(&migrations, 2, "THIS IS INVALID SQL;")?;
        assert!(database_schema::generate(Some(&url), &migrations, &output)
            .await
            .is_err());
        assert_eq!(std::fs::read_to_string(&output)?, dump);
        Ok(())
    }
    .await;
    #[cfg(feature = "mysql")]
    {
        admin(&format!("DROP DATABASE {database}")).await?;
        admin(&format!("DROP USER '{database}'@'%'")).await?;
    }
    #[cfg(feature = "postgres")]
    {
        admin(&format!("DROP DATABASE {database} WITH (FORCE)")).await?;
        admin(&format!("DROP ROLE {database}")).await?;
    }
    result
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_service_child() -> TestResult {
    let Some(url) = std::env::var_os("DATABASE_SCHEMA_TEST_URL") else {
        return Ok(());
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(database_schema::generate(
        Some(&url.to_string_lossy()),
        std::env::var_os("DATABASE_SCHEMA_TEST_MIGRATIONS").unwrap(),
        std::env::var_os("DATABASE_SCHEMA_TEST_OUTPUT").unwrap(),
    ))?;
    Ok(())
}
