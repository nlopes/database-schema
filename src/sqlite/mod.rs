//! Implementation of the `sqlite` feature

// Tables and views must exist before their indexes and triggers are restored.
const SQLITE_SCHEMA_QUERY: &str = r"SELECT name, type as mytype, sql
FROM sqlite_schema
WHERE
  sql NOTNULL AND
  name NOT LIKE 'sqlite\_%' ESCAPE '\'
ORDER BY CASE type WHEN 'table' THEN 0 WHEN 'view' THEN 1 ELSE 2 END,
  tbl_name, type, name";

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
