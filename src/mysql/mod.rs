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

    let mut migration_url = url::Url::parse(connection_url)
        .map_err(|error| Error::UriConfiguration(error.to_string()))?;
    let query = migration_url
        .query_pairs()
        .map(|(key, value)| {
            #[cfg(feature = "sqlx")]
            let socket_key = "socket";
            #[cfg(feature = "diesel")]
            let socket_key = "unix_socket";
            let key = if matches!(&*key, "socket" | "unix_socket") {
                socket_key.into()
            } else {
                key
            };
            (key.into_owned(), value.into_owned())
        })
        .collect::<Vec<_>>();
    if !query.is_empty() {
        migration_url.query_pairs_mut().clear().extend_pairs(query);
    }
    migrate(migration_url.as_str(), migrations_path).await?;

    let (mut cmd, _credentials) = dump_command(&options, destination_path.as_ref())?;
    crate::process::run(&mut cmd).await?;
    Ok(())
}

#[allow(unused_results)]
fn dump_command(
    options: &MySqlConnectOptions,
    destination_path: &std::path::Path,
) -> Result<(std::process::Command, Option<tempfile::NamedTempFile>), Error> {
    use std::io::Write;

    let mut cmd = std::process::Command::new("mysqldump");
    let credentials = if let Some(password) = &options.password {
        let mut file = tempfile::NamedTempFile::new()?;
        let escaped = password
            .chars()
            .map(|c| match c {
                '\\' => "\\\\".to_owned(),
                '"' => "\\\"".to_owned(),
                '\n' => "\\n".to_owned(),
                '\r' => "\\r".to_owned(),
                '\t' => "\\t".to_owned(),
                '\u{8}' => "\\b".to_owned(),
                c => c.to_string(),
            })
            .collect::<String>();
        writeln!(file, "[client]\npassword=\"{escaped}\"")?;
        file.flush()?;
        // MySQL reads this option only before the other command-line options.
        let mut argument = std::ffi::OsString::from("--defaults-extra-file=");
        argument.push(file.path());
        let _ = cmd.arg(argument);
        Some(file)
    } else {
        None
    };
    cmd.arg("--no-data")
        .arg("--routines")
        .arg("--skip-comments")
        .arg("--set-gtid-purged=OFF")
        .arg("--result-file")
        .arg(destination_path)
        .arg("--host")
        .arg(options.host.clone())
        .arg("--port")
        .arg(options.port.to_string())
        .arg("--user")
        .arg(options.username.clone())
        .arg("--ssl-mode")
        .arg(format!("{}", options.ssl_mode));

    if let Some(ref socket) = options.socket {
        cmd.arg("--protocol=SOCKET").arg("--socket").arg(socket);
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
        cmd.arg("--").arg(database);
    } else {
        cmd.arg("--all-databases");
    }

    Ok((cmd, credentials))
}

fn extract_connect_options(connection_url: &str) -> Result<MySqlConnectOptions, Error> {
    let url = url::Url::parse(connection_url)
        .map_err(|error| Error::UriConfiguration(error.to_string()))?;
    if url.scheme() != "mysql" {
        return Err(Error::UriConfiguration("expected a mysql URL".into()));
    }

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
            &percent_decode_str(username)
                .decode_utf8()
                .map_err(Error::UriConfigurationDecoding)?,
        );
    }

    if let Some(password) = url.password() {
        options = options.password(
            &percent_decode_str(password)
                .decode_utf8()
                .map_err(Error::UriConfigurationDecoding)?,
        );
    }

    let path = url.path().trim_start_matches('/');
    if !path.is_empty() {
        options = options.database(&percent_decode_str(path).decode_utf8()?);
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

            "socket" | "unix_socket" => {
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
    use sqlx::{migrate::Migrator, mysql::MySqlConnectOptions, ConnectOptions};
    use std::str::FromStr;

    let mut conn = MySqlConnectOptions::from_str(connection_url)?
        .connect()
        .await?;

    let migrator = Migrator::new(migrations_path.as_ref()).await?;
    migrator.run(&mut conn).await?;
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
        .map_err(Error::MigrationRunError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_urls_return_errors() {
        for url in [
            "not-a-url",
            "mysql://host:invalid/db",
            "postgres://host/db",
            "mysql://user:%FF@host/db",
        ] {
            assert!(super::extract_connect_options(url).is_err());
        }
    }

    #[test]
    fn dump_uses_private_credentials_and_socket_aliases() -> Result<(), crate::Error> {
        for alias in ["socket", "unix_socket"] {
            let options = super::extract_connect_options(&format!(
                "mysql://test:secret%3A%2B%5C%22@localhost/my%20database?{alias}=%2Ftmp%2Fmysql.sock"
            ))?;
            let (command, credentials) =
                super::dump_command(&options, std::path::Path::new("structure.sql"))?;
            let args = command
                .get_args()
                .map(|a| a.to_string_lossy())
                .collect::<Vec<_>>();
            assert!(args.iter().all(|a| !a.contains("secret")));
            assert!(args[0].starts_with("--defaults-extra-file="));
            assert!(args.iter().any(|a| *a == "--protocol=SOCKET"));
            assert!(args.iter().any(|a| *a == "/tmp/mysql.sock"));
            assert_eq!(args.last().unwrap(), "my database");
            let credentials = credentials.unwrap();
            assert!(std::fs::read_to_string(credentials.path())?.starts_with("[client]\npassword="));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    credentials.as_file().metadata()?.permissions().mode() & 0o777,
                    0o600
                );
            }
            let path = credentials.path().to_owned();
            drop(credentials);
            assert!(!path.exists());
        }
        Ok(())
    }

    #[cfg(all(feature = "sqlx", feature = "mysql"))]
    #[tokio::test]
    async fn test_write_structure_sql() -> Result<(), crate::error::Error> {
        let root = tempfile::tempdir()?;
        let destination_path = root.path().join("sqlx-mysql-structure.sql");
        let migrations_path = std::path::PathBuf::from("./fixtures/sqlx/mysql/migrations");
        let connection_url = std::env::var("DATABASE_SCHEMA_TEST_MYSQL_URL")
            .unwrap_or_else(|_| super::DEFAULT_CONNECTION_URL.into());
        super::write_structure_sql(&connection_url, migrations_path, &destination_path).await?;
        let contents = std::fs::read_to_string(destination_path)?;

        assert!(contents.contains("CREATE TABLE `sqlx_users` (\n  `id` varchar(32) NOT NULL,\n  `email` text NOT NULL,\n  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;\n"));
        Ok(())
    }

    #[cfg(all(feature = "diesel", feature = "mysql"))]
    #[tokio::test]
    async fn test_write_structure_sql() -> Result<(), crate::error::Error> {
        let root = tempfile::tempdir()?;
        let destination_path = root.path().join("diesel-mysql-structure.sql");
        let migrations_path = std::path::PathBuf::from("./fixtures/diesel/mysql/migrations");
        let connection_url = std::env::var("DATABASE_SCHEMA_TEST_MYSQL_URL")
            .unwrap_or_else(|_| super::DEFAULT_CONNECTION_URL.into());
        super::write_structure_sql(&connection_url, migrations_path, &destination_path).await?;
        let contents = std::fs::read_to_string(destination_path)?;

        assert!(contents.contains("CREATE TABLE `diesel_users` (\n  `id` varchar(32) NOT NULL,\n  `email` text NOT NULL,\n  `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;\n"));
        Ok(())
    }
}
