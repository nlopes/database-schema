//! Implementation of the `sqlite` feature

// The below select query was taken almost verbatim out of sqlite3 source code.
// See https://sqlite.org/src/file?ci=trunk&name=src/shell.c.in&ln=10008 and also
// the generated shell.c in this mirror
// https://github.com/smparkes/sqlite/blob/8caf9219240123fbe6cff67b1e0da778c62d7621/src/shell.c#L2063
// (they are somewhat out of sync but the query is the same)
const SQLITE_SCHEMA_QUERY: &str = "SELECT name, type as mytype, sql
FROM sqlite_schema
WHERE
  sql NOTNULL AND
  name NOT LIKE 'sqlite_%'
ORDER BY tbl_name, type DESC, name";

#[cfg(any(feature = "sqlx", feature = "diesel"))]
pub(crate) async fn write_structure_sql<P: AsRef<std::path::Path>, Q: AsRef<std::path::Path>>(
    connection_url: &str,
    migrations_path: P,
    destination_path: Q,
) -> Result<(), crate::error::Error> {
    let structure_sql = fetch_structure_sql(connection_url, migrations_path).await?;
    Ok(std::fs::write(destination_path, structure_sql)?)
}

#[cfg(feature = "sqlx")]
mod sqlx;
#[cfg(feature = "sqlx")]
use crate::sqlite::sqlx::fetch_structure_sql;
#[cfg(feature = "sqlx")]
pub(crate) use crate::sqlite::sqlx::DEFAULT_CONNECTION_URL;

#[cfg(feature = "diesel")]
mod diesel;
#[cfg(feature = "diesel")]
use diesel::fetch_structure_sql;
#[cfg(feature = "diesel")]
pub(crate) use diesel::DEFAULT_CONNECTION_URL;
