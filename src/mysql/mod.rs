//! Implementation of the `mysql` feature

pub(crate) const DEFAULT_CONNECTION_URL: &str = "mysql://root:@127.0.0.1:3306/mysql";

use percent_encoding::percent_decode_str;

use crate::error::Error;

mod options;

use options::MySqlConnectOptions;

#[allow(unused_results)]
#[cfg(any(feature = "sqlx", feature = "diesel"))]
pub(crate) async fn write_structure_sql<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
    destination_path: Q,
) -> Result<(), Error> {
    let options = extract_connect_options(connection_url)?;

    migrate(connection_url, migrations_path).await?;

    let mut cmd = std::process::Command::new("mysqldump");
    cmd.arg("--no-data")
        .arg("--routines")
        .arg("--skip-comments")
        .arg("--result-file")
        .arg(destination_path.as_ref())
        .arg("--host")
        .arg(options.host.clone())
        .arg("--port")
        .arg(options.port.to_string())
        .arg("--user")
        .arg(options.username.clone())
        .arg("--ssl-mode")
        .arg(format!("{}", options.ssl_mode));

    if let Some(ref password) = options.password {
        cmd.arg("--password").arg(password.clone());
    }

    if let Some(ref ssl_ca) = options.ssl_ca {
        cmd.arg("--ssl-ca").arg(ssl_ca.clone());
    }
    if let Some(ref ssl_cert) = options.ssl_client_cert {
        cmd.arg("--ssl-cert").arg(ssl_cert.clone());
    }
    if let Some(ref ssl_key) = options.ssl_client_key {
        cmd.arg("--ssl-key").arg(ssl_key.clone());
    }
    // This must come last because mysqldump expects the database name to be the last
    // argument
    if let Some(ref database) = options.database {
        cmd.arg(database.clone());
    } else {
        cmd.arg("--all-databases");
    }

    crate::process::run(&mut cmd).await?;
    Ok(())
}

fn extract_connect_options(connection_url: &str) -> Result<MySqlConnectOptions, Error> {
    let url = url::Url::parse(connection_url).unwrap();

    let mut options = MySqlConnectOptions::new();

    if let Some(host) = url.host_str() {
        options = options.host(host);
    }

    if let Some(port) = url.port() {
        options = options.port(port);
    }

    let username = url.username();
    if !username.is_empty() {
        options = options.username(
            &*percent_decode_str(username)
                .decode_utf8()
                .map_err(Error::UriConfigurationDecoding)?,
        );
    }

    if let Some(password) = url.password() {
        options = options.password(
            &*percent_decode_str(password)
                .decode_utf8()
                .map_err(Error::UriConfigurationDecoding)?,
        );
    }

    let path = url.path().trim_start_matches('/');
    if !path.is_empty() {
        options = options.database(path);
    }

    for (key, value) in url.query_pairs().into_iter() {
        match &*key {
            "sslmode" | "ssl-mode" | "ssl_mode" => {
                options = options.ssl_mode(value.parse().map_err(Error::UriConfiguration)?);
            }

            "sslca" | "ssl-ca" | "ssl_ca" => {
                options = options.ssl_ca(&*value);
            }

            "sslcert" | "ssl-cert" | "ssl_cert" => options = options.ssl_client_cert(&*value),

            "sslkey" | "ssl-key" | "ssl_key" => options = options.ssl_client_key(&*value),

            "socket" => {
                options = options.socket(&*value);
            }

            _ => {}
        }
    }

    Ok(options)
}

#[cfg(feature = "sqlx")]
async fn migrate<P: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
) -> Result<(), sqlx::Error> {
    use sqlx::{
        migrate::{Migrate, Migrator},
        mysql::MySqlConnectOptions,
        ConnectOptions,
    };
    use std::str::FromStr;

    let mut conn = MySqlConnectOptions::from_str(connection_url)?
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
    let mut conn = diesel::MysqlConnection::establish(connection_url)?;
    let migrations = FileBasedMigrations::from_path(migrations_path)?;
    let _ = HarnessWithOutput::write_to_stdout(&mut conn)
        .run_pending_migrations(migrations)
        .map(|_| ());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "sqlx", feature = "mysql"))]
    #[tokio::test]
    async fn test_write_structure_sql() -> Result<(), crate::error::Error> {
        let destination_path = std::env::temp_dir().join("sqlx-mysql-structure.sql");
        let migrations_path = std::path::PathBuf::from("./fixtures/sqlx/mysql/migrations");
        let _ = super::write_structure_sql(
            super::DEFAULT_CONNECTION_URL,
            migrations_path,
            &destination_path,
        )
        .await?;
        let contents = std::fs::read_to_string(destination_path)?;

        assert!(contents.contains("CREATE TABLE `sqlx_users` (\n  `id` varchar(32) NOT NULL,\n  `email` text NOT NULL,\n  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;\n"));
        Ok(())
    }

    #[cfg(all(feature = "diesel", feature = "mysql"))]
    #[tokio::test]
    async fn test_write_structure_sql() -> Result<(), crate::error::Error> {
        let destination_path = std::env::temp_dir().join("diesel-mysql-structure.sql");
        let migrations_path = std::path::PathBuf::from("./fixtures/diesel/mysql/migrations");
        let _ = super::write_structure_sql(
            super::DEFAULT_CONNECTION_URL,
            migrations_path,
            &destination_path,
        )
        .await?;
        let contents = std::fs::read_to_string(destination_path)?;

        assert!(contents.contains("CREATE TABLE `diesel_users` (\n  `id` varchar(32) NOT NULL,\n  `email` text NOT NULL,\n  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;\n"));
        Ok(())
    }
}
