#![cfg(any(feature = "mysql", feature = "postgres"))]

use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(feature = "mysql")]
const ADMIN_URL: &str = "mysql://root:@127.0.0.1:3306/mysql";
#[cfg(feature = "postgres")]
const ADMIN_URL: &str = "postgresql://root:@127.0.0.1:5432/postgres";

async fn admin(sql: &str) -> TestResult {
    execute(ADMIN_URL, sql).await
}

async fn execute(url: &str, sql: &str) -> TestResult {
    #[cfg(feature = "sqlx")]
    {
        use sqlx::{Connection, Executor};
        #[cfg(feature = "mysql")]
        let mut conn = sqlx::MySqlConnection::connect(url).await?;
        #[cfg(feature = "postgres")]
        let mut conn = sqlx::PgConnection::connect(url).await?;
        let _ = conn
            .execute(sqlx::raw_sql(sqlx::AssertSqlSafe(sql)))
            .await?;
    }
    #[cfg(feature = "diesel")]
    {
        use diesel::{connection::SimpleConnection, Connection};
        #[cfg(feature = "mysql")]
        let mut conn = diesel::MysqlConnection::establish(url)?;
        #[cfg(feature = "postgres")]
        let mut conn = diesel::PgConnection::establish(url)?;
        conn.batch_execute(sql)?;
    }
    Ok(())
}

async fn scalar(url: &str, sql: &str) -> Result<i64, Box<dyn std::error::Error>> {
    #[cfg(feature = "sqlx")]
    {
        use sqlx::Connection;
        #[cfg(feature = "mysql")]
        let mut conn = sqlx::MySqlConnection::connect(url).await?;
        #[cfg(feature = "postgres")]
        let mut conn = sqlx::PgConnection::connect(url).await?;
        Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_one(&mut conn)
            .await?)
    }
    #[cfg(feature = "diesel")]
    {
        use diesel::{Connection, RunQueryDsl};
        #[derive(diesel::QueryableByName)]
        struct Value(#[diesel(sql_type = diesel::sql_types::BigInt, column_name = value)] i64);
        #[cfg(feature = "mysql")]
        let mut conn = diesel::MysqlConnection::establish(url)?;
        #[cfg(feature = "postgres")]
        let mut conn = diesel::PgConnection::establish(url)?;
        Ok(diesel::sql_query(sql).get_result::<Value>(&mut conn)?.0)
    }
}

async fn recreate(database: &str) -> TestResult {
    #[cfg(feature = "mysql")]
    {
        admin(&format!("DROP DATABASE {database}")).await?;
        admin(&format!("CREATE DATABASE {database}")).await?;
    }
    #[cfg(feature = "postgres")]
    {
        admin(&format!("DROP DATABASE {database} WITH (FORCE)")).await?;
        admin(&format!("CREATE DATABASE {database} OWNER {database}")).await?;
    }
    Ok(())
}

fn restore(database: &str, dump: &Path) -> TestResult {
    #[cfg(feature = "postgres")]
    let output = std::process::Command::new("psql")
        .args(["-X", "--set=ON_ERROR_STOP=1", "--dbname"])
        .arg(format!("postgresql://root:@127.0.0.1:5432/{database}"))
        .arg("--command")
        .arg(format!("SET ROLE {database}"))
        .arg("--file")
        .arg(dump)
        .output()?;
    #[cfg(feature = "mysql")]
    let output = std::process::Command::new("mysql")
        .args([
            "--host=127.0.0.1",
            "--port=3306",
            "--user=root",
            "--database",
        ])
        .arg(database)
        .stdin(std::fs::File::open(dump)?)
        .output()?;
    assert!(
        output.status.success(),
        "restore failed: {}{}\n\ndump:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        std::fs::read_to_string(dump)?
    );
    Ok(())
}

async fn verify_restored_schema(url: &str) -> TestResult {
    for table in ["schema_users", "schema_posts", "schema_audit"] {
        assert_eq!(
            scalar(url, &format!("SELECT COUNT(*) AS value FROM {table}")).await?,
            0
        );
    }
    execute(
        url,
        "INSERT INTO schema_users (email) VALUES ('restored@example.org');",
    )
    .await?;
    assert_eq!(
        scalar(
            url,
            "SELECT COUNT(*) AS value FROM schema_users WHERE id > 0 AND created_at IS NOT NULL"
        )
        .await?,
        1
    );
    assert_eq!(
        scalar(
            url,
            "SELECT COUNT(*) AS value FROM schema_user_emails WHERE email = 'restored@example.org'"
        )
        .await?,
        1
    );
    assert_eq!(
        scalar(url, "SELECT COUNT(*) AS value FROM schema_audit").await?,
        1
    );
    assert!(
        execute(url, "INSERT INTO schema_posts (user_id) VALUES (-1);")
            .await
            .is_err()
    );
    execute(
        url,
        "INSERT INTO schema_posts (user_id) SELECT id FROM schema_users;",
    )
    .await?;
    assert_eq!(
        scalar(
            url,
            "SELECT COUNT(*) AS value FROM schema_posts WHERE id > 0"
        )
        .await?,
        1
    );
    #[cfg(feature = "postgres")]
    {
        assert_eq!(scalar(url, "SELECT schema_user_count() AS value").await?, 1);
        assert_eq!(scalar(url, "SELECT COUNT(*) AS value FROM pg_indexes WHERE schemaname = 'public' AND tablename = 'schema_users' AND indexname = 'schema_users_email'").await?, 1);
    }
    #[cfg(feature = "mysql")]
    {
        // Diesel's prepared statements cannot read a CALL result set.
        let count = std::process::Command::new("mysql")
            .args([
                "--host=127.0.0.1",
                "--port=3306",
                "--user=root",
                "--batch",
                "--skip-column-names",
                "--database",
            ])
            .arg(url::Url::parse(url)?.path().trim_start_matches('/'))
            .args(["--execute", "CALL schema_user_count()"])
            .output()?;
        assert!(
            count.status.success(),
            "{}",
            String::from_utf8_lossy(&count.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&count.stdout).trim(), "1");
        assert_eq!(scalar(url, "SELECT COUNT(*) AS value FROM information_schema.statistics WHERE table_schema = DATABASE() AND table_name = 'schema_users' AND index_name = 'schema_users_email'").await?, 1);
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

fn comparable_schema(dump: &str) -> String {
    #[cfg(feature = "mysql")]
    {
        dump.to_owned()
    }
    #[cfg(feature = "postgres")]
    {
        let mut restrict_keys = Vec::new();
        let mut unrestrict_keys = Vec::new();
        let mut schema = String::new();
        for line in dump.split_inclusive('\n') {
            if let Some(key) = line.strip_prefix("\\restrict ") {
                restrict_keys.push(key.trim_end());
                schema.push_str("\\restrict <restore-key>\n");
            } else if let Some(key) = line.strip_prefix("\\unrestrict ") {
                unrestrict_keys.push(key.trim_end());
                schema.push_str("\\unrestrict <restore-key>\n");
            } else {
                schema.push_str(line);
            }
        }
        assert_eq!(restrict_keys, unrestrict_keys);
        assert!(restrict_keys.len() <= 1);
        assert!(restrict_keys.iter().all(|key| !key.is_empty()));
        schema
    }
}

#[tokio::test]
async fn authenticated_dump_restores_and_repeats() -> TestResult {
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
        #[cfg(feature = "mysql")]
        let schema = include_str!("../fixtures/server/mysql/schema.sql");
        #[cfg(feature = "postgres")]
        let schema = include_str!("../fixtures/server/postgres/schema.sql");
        migration(&migrations, 1, schema)?;
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
        let expected = comparable_schema(&dump);
        for destination in [root.path().join("another-structure.sql"), output.clone()] {
            database_schema::generate(Some(&url), &migrations, &destination).await?;
            assert_eq!(comparable_schema(&std::fs::read_to_string(destination)?), expected);
        }
        recreate(&database).await?;
        database_schema::generate(Some(&url), &migrations, &output).await?;
        let dump = std::fs::read_to_string(&output)?;
        assert_eq!(comparable_schema(&dump), expected);
        migration(&migrations, 2, "THIS IS INVALID SQL;")?;
        assert!(database_schema::generate(Some(&url), &migrations, &output)
            .await
            .is_err());
        assert_eq!(std::fs::read_to_string(&output)?, dump);
        recreate(&database).await?;
        restore(&database, &output)?;
        verify_restored_schema(&url).await?;
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
