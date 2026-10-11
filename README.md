# database-schema

[![CI Status](https://github.com/nlopes/database-schema/workflows/Test/badge.svg)](https://github.com/nlopes/database-schema/actions)
[![docs.rs](https://docs.rs/database-schema/badge.svg)](https://docs.rs/database-schema)
[![crates.io](https://img.shields.io/crates/v/database-schema.svg)](https://crates.io/crates/database-schema)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/nlopes/database-schema/blob/master/LICENSE)

Generate a SQL schema file from database migrations. This crate takes inspiration
from the Ruby on Rails [schema dump].

## Features

Select exactly one database (`sqlite`, `mysql`, or `postgres`) and one migration
driver (`sqlx` or `diesel`). SQLx also requires a runtime feature.

| Database | Driver | Runtime |
|----------|--------|---------|
| `sqlite`, `mysql`, or `postgres` | `sqlx` | `runtime-async-std` or `runtime-tokio` |
| `sqlite`, `mysql`, or `postgres` | `diesel` | Optional; required for macros |

The `native-tls` feature is enabled by default. To use Rustls with SQLx, disable
default features and enable `rustls`.

```toml
[dependencies]
database-schema = { version = "0.2", features = ["sqlite", "sqlx", "runtime-tokio"] }
```

For Diesel:

```toml
[dependencies]
database-schema = { version = "0.2", features = ["sqlite", "diesel"] }
```

## Usage

Run migrations, then write the database structure to a SQL file. The async API
requires an executor.

```rust,no_run
use database_schema::generate;

# async fn example() -> Result<(), database_schema::Error> {
generate(None, "./migrations", "structure.sql").await?;
# Ok(())
# }
```

SQLite uses an in-memory database by default. MySQL and PostgreSQL require a
running server and the `mysqldump` or `pg_dump` executable. Use a database reserved
for schema generation: migrations change its structure.

## Macros

Enable `macros` and a runtime feature to generate a schema without managing an
executor. If both runtimes are enabled, the macros use Tokio. They return `()`
and panic if schema generation fails. With `runtime-tokio`, call them outside an
active Tokio runtime.

For a `build.rs` script, add the crate as a build dependency:

```toml
[build-dependencies]
database-schema = { version = "0.2", features = ["sqlite", "diesel", "macros", "runtime-tokio"] }
```

```rust,no_run
# #[cfg(feature = "macros")]
# {
use database_schema::generate_without_runtime;

generate_without_runtime!("./migrations", "structure.sql");
# }
```

To use `./migrations` and `./structure.sql`:

```rust,no_run
# #[cfg(feature = "macros")]
# {
database_schema::generate_without_runtime_using_defaults!();
# }
```

The macros write the file when called. A call from `build.rs` writes it during
the build.

## Customization

```rust,no_run
use database_schema::DatabaseSchemaBuilder;

# async fn example() -> Result<(), database_schema::Error> {
DatabaseSchemaBuilder::new()
    .migrations_dir("db/migrations")?
    .destination_path("db/structure.sql")
    .build()
    .dump()
    .await?;
# Ok(())
# }
```

The builder defaults to `./migrations` and `./structure.sql`. Use
`connection_url` to set the connection URL for MySQL or PostgreSQL. Migration
failures return an error before the dump file is written.

Connection URLs are redacted in `Debug` output. Dump passwords are kept out of
command arguments and tracing logs. MySQL uses a temporary client option file;
PostgreSQL uses a temporary password file. PostgreSQL passwords with line breaks
use the child process environment because its password file format cannot store
them. MySQL also reads the usual client option files, which can override the
temporary password option.

## Repeated generation

With unchanged database state and the same client and server versions, SQLite
and MySQL dumps are byte-identical across repeated generation. Output paths and
migration execution timestamps are not embedded in the schema output.

MySQL schema dumps omit GTID replication positions. These positions describe
server-wide transaction history and can change without a schema change.

Recent PostgreSQL clients add random `\restrict` and `\unrestrict` keys to
protect restoration with `psql`. These keys change between dumps; the rest of the
schema output stays the same. The crate keeps this protection. PostgreSQL
[does not recommend a fixed key for general use][restrict key]. Client or server
upgrades can also change dump headers or SQL formatting.

## Local checks

Run database tests with the isolated test services:

```sh
docker compose up -d --wait
cargo test --features mysql,diesel --all-targets
cargo test --features postgres,sqlx,runtime-tokio --all-targets
docker compose down
```

The tests create temporary databases and accounts. PostgreSQL requires a password
for those accounts; the local `root` account uses trust authentication.

See `.github/workflows/ci.yml` for the complete feature matrix. Run its feature
configurations separately; `--all-features` combines incompatible database and
driver features.

[schema dump]: https://guides.rubyonrails.org/active_record_migrations.html#schema-dumping-and-you
[restrict key]: https://www.postgresql.org/docs/14/app-pgdump.html#APP-PGDUMP-OPTIONS
