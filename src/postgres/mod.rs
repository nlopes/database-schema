//! Implementation of the `postgres` feature

pub(crate) const DEFAULT_CONNECTION_URL: &str = "postgresql://root:@127.0.0.1:5432/postgres";

use crate::error::Error;

#[allow(unused_results)]
#[cfg(any(feature = "sqlx", feature = "diesel"))]
pub(crate) async fn write_structure_sql<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
    destination_path: Q,
) -> Result<(), Error> {
    migrate(connection_url, migrations_path).await?;

    let (mut cmd, _credentials) = dump_command(connection_url, destination_path.as_ref())?;
    crate::process::run(&mut cmd).await?;
    Ok(())
}

fn dump_command(
    connection_url: &str,
    destination_path: &std::path::Path,
) -> Result<(std::process::Command, Option<tempfile::NamedTempFile>), Error> {
    use std::io::Write;
    let (mut connection, password) = remove_password(connection_url)?;
    let mut cmd = std::process::Command::new("pg_dump");
    let credentials = if let Some(password) = password {
        let _ = cmd.env_remove("PGPASSWORD");
        if password.contains(['\n', '\r']) {
            // The line-based .pgpass format cannot represent these passwords.
            let _ = cmd.env("PGPASSWORD", password);
            None
        } else {
            let mut file = tempfile::NamedTempFile::new()?;
            let escaped = password.replace('\\', "\\\\").replace(':', "\\:");
            writeln!(file, "*:*:*:*:{escaped}")?;
            file.flush()?;
            let path = file.path().to_string_lossy();
            if connection.starts_with("postgres://") || connection.starts_with("postgresql://") {
                let separator = if connection.contains('?') { '&' } else { '?' };
                connection.push(separator);
                // An explicit empty password prevents service settings from replacing it.
                connection.push_str("password=&passfile=");
                connection.push_str(
                    &percent_encoding::utf8_percent_encode(
                        &path,
                        percent_encoding::NON_ALPHANUMERIC,
                    )
                    .to_string(),
                );
            } else {
                connection.push_str(" password='' passfile='");
                connection.push_str(&path.replace('\\', "\\\\").replace('\'', "\\'"));
                connection.push('\'');
            }
            Some(file)
        }
    } else {
        None
    };
    let _ = cmd
        .arg("--schema-only")
        .arg("--no-owner")
        .arg("--no-privileges")
        .arg("--file")
        .arg(destination_path)
        .arg(connection);
    Ok((cmd, credentials))
}

fn remove_password(connection: &str) -> Result<(String, Option<String>), Error> {
    use percent_encoding::percent_decode_str;
    if connection.starts_with("postgres://") || connection.starts_with("postgresql://") {
        let (base, query) = connection.split_once('?').unwrap_or((connection, ""));
        let mut base = base.to_owned();
        let mut password = None;
        let authority_start = base.find("://").expect("URI prefix checked") + 3;
        let authority_end = base[authority_start..]
            .find('/')
            .map_or(base.len(), |i| authority_start + i);
        if let Some(at) = base[authority_start..authority_end].rfind('@') {
            let at = authority_start + at;
            if let Some(colon) = base[authority_start..at].find(':') {
                let colon = authority_start + colon;
                password = Some(
                    percent_decode_str(&base[colon + 1..at])
                        .decode_utf8()?
                        .into_owned(),
                );
                base.replace_range(colon..at, "");
            }
        }
        let mut kept = Vec::new();
        for parameter in query.split('&').filter(|p| !p.is_empty()) {
            let (key, value) = parameter.split_once('=').unwrap_or((parameter, ""));
            if percent_decode_str(key).decode_utf8()? == "password" {
                password = Some(percent_decode_str(value).decode_utf8()?.into_owned());
            } else {
                kept.push(parameter);
            }
        }
        if !kept.is_empty() {
            base.push('?');
            base.push_str(&kept.join("&"));
        }
        return Ok((base, password));
    }
    if !connection.contains('=') {
        return Ok((connection.into(), None));
    }

    // Diesel also accepts libpq keyword strings with quoted and escaped values.
    let mut input = connection.chars().peekable();
    let mut parameters = Vec::new();
    let mut password = None;
    while input.peek().is_some() {
        while input.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            let _ = input.next();
        }
        if input.peek().is_none() {
            break;
        }
        let mut key = String::new();
        while input
            .peek()
            .is_some_and(|c| *c != '=' && !c.is_ascii_whitespace())
        {
            key.push(input.next().expect("peeked character"));
        }
        while input.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            let _ = input.next();
        }
        if key.is_empty() || input.next() != Some('=') {
            return Err(Error::UriConfiguration(
                "invalid PostgreSQL connection string".into(),
            ));
        }
        while input.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            let _ = input.next();
        }
        let quoted = input.peek() == Some(&'\'');
        if quoted {
            let _ = input.next();
        }
        let mut value = String::new();
        let mut closed = !quoted;
        while let Some(c) = input.next() {
            if c == '\\' {
                let escaped = input.next().ok_or_else(|| {
                    Error::UriConfiguration("invalid PostgreSQL connection string".into())
                })?;
                value.push(escaped);
            } else if (quoted && c == '\'') || (!quoted && c.is_ascii_whitespace()) {
                closed = true;
                break;
            } else {
                value.push(c);
            }
        }
        if !closed {
            return Err(Error::UriConfiguration(
                "invalid PostgreSQL connection string".into(),
            ));
        }
        if key == "password" {
            password = Some(value);
        } else {
            parameters.push(format!(
                "{key}='{}'",
                value.replace('\\', "\\\\").replace('\'', "\\'")
            ));
        }
    }
    Ok((parameters.join(" "), password))
}

#[cfg(feature = "sqlx")]
async fn migrate<P: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
) -> Result<(), sqlx::Error> {
    use sqlx::{migrate::Migrator, postgres::PgConnectOptions, ConnectOptions};
    use std::str::FromStr;

    let mut conn = PgConnectOptions::from_str(connection_url)?
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
    let mut conn = diesel::PgConnection::establish(connection_url)?;
    let migrations = FileBasedMigrations::from_path(migrations_path)?;
    let _ = HarnessWithOutput::write_to_stdout(&mut conn)
        .run_pending_migrations(migrations)
        .map_err(Error::MigrationRunError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn removes_passwords_without_changing_uri_options() -> Result<(), crate::Error> {
        let (connection, password) = super::remove_password(
            "postgresql://user:old%3Asecret@host1:5432,host2:5433/database?sslmode=require&password=new+secret%3A%5C&application_name=test%20app"
        )?;
        assert_eq!(password.as_deref(), Some("new+secret:\\"));
        assert_eq!(connection, "postgresql://user@host1:5432,host2:5433/database?sslmode=require&application_name=test%20app");
        Ok(())
    }

    #[test]
    fn removes_passwords_from_libpq_keyword_strings() -> Result<(), crate::Error> {
        let (connection, password) = super::remove_password(
            "host = localhost user=test password='old secret' dbname='test database' password = 'new\\'secret\\\\' options='-c search_path=public'"
        )?;
        assert_eq!(password.as_deref(), Some("new'secret\\"));
        assert_eq!(
            connection,
            "host='localhost' user='test' dbname='test database' options='-c search_path=public'"
        );
        let (_, password) = super::remove_password("password==secret host=localhost")?;
        assert_eq!(password.as_deref(), Some("=secret"));
        for malformed in [
            "password='unclosed",
            "password=trailing\\",
            "password secret host=localhost",
        ] {
            assert!(super::remove_password(malformed).is_err());
        }
        Ok(())
    }

    #[test]
    fn dump_uses_private_password_file() -> Result<(), crate::Error> {
        let (command, credentials) = super::dump_command(
            "postgresql://test:secret%3A%5C@localhost/test",
            std::path::Path::new("structure.sql"),
        )?;
        assert!(command
            .get_args()
            .all(|a| !a.to_string_lossy().contains("secret")));
        let credentials = credentials.unwrap();
        assert_eq!(
            std::fs::read_to_string(credentials.path())?,
            "*:*:*:*:secret\\:\\\\\n"
        );
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
        Ok(())
    }

    #[test]
    fn newline_passwords_stay_out_of_arguments() -> Result<(), crate::Error> {
        let (command, credentials) = super::dump_command(
            "postgresql://test:secret%0Aline@localhost/test",
            std::path::Path::new("structure.sql"),
        )?;
        assert!(credentials.is_none());
        assert!(command
            .get_args()
            .all(|a| !a.to_string_lossy().contains("secret")));
        assert!(command.get_envs().any(|(key, value)| key == "PGPASSWORD"
            && value == Some(std::ffi::OsStr::new("secret\nline"))));
        Ok(())
    }

    async fn call_write_structure_sql<M: AsRef<str>, D: AsRef<str>>(
        fixtures_path: M,
        destination_filename: D,
    ) -> Result<(), crate::error::Error> {
        let root = tempfile::tempdir()?;
        let destination_path = root.path().join(destination_filename.as_ref());
        let migrations_path = std::path::PathBuf::from(fixtures_path.as_ref()).join("migrations");
        super::write_structure_sql(
            super::DEFAULT_CONNECTION_URL,
            migrations_path,
            &destination_path,
        )
        .await?;
        let expected = std::fs::read_to_string(format!(
            "./{}/{}",
            fixtures_path.as_ref(),
            destination_filename.as_ref()
        ))?;
        let contents = std::fs::read_to_string(destination_path)?;
        let contents = normalize_dump_output(&contents);
        let expected = normalize_dump_output(&expected);

        for expected_fragment in expected
            .split("\n\n")
            .filter(|chunk| !chunk.trim().is_empty())
        {
            assert!(
                contents.contains(expected_fragment),
                "missing expected pg_dump fragment:\n\n{expected_fragment}\n\nactual dump:\n\n{contents}"
            );
        }
        Ok(())
    }

    fn normalize_dump_output(dump: &str) -> String {
        dump.lines()
            .filter(|line| {
                !line.starts_with("\\restrict ")
                    && !line.starts_with("\\unrestrict ")
                    && *line != "SET transaction_timeout = 0;"
            })
            .collect::<Vec<_>>()
            .join("\n")
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
