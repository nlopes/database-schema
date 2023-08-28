# database-schema

[![CI Status](https://github.com/nlopes/database-schema/workflows/Test/badge.svg)](https://github.com/nlopes/database-schema/actions)
[![docs.rs](https://docs.rs/database-schema/badge.svg)](https://docs.rs/database-schema)
[![crates.io](https://img.shields.io/crates/v/database-schema.svg)](https://crates.io/crates/database-schema)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/nlopes/database-schema/blob/master/LICENSE)

This crate provides a simple way to dump a database structure to a file, in SQL
format.

It takes inspiration by the ruby on rails [schema dump].

## Usage

```rust
use std::path::PathBuf;

database_schema::generate_without_runtime_using_defaults!();
```


## Feature flags

`database-schema` uses a set of [feature flags] to reduce the size of the libray and
therefore your binary. The way one should use this package is to pick the right
combination of feature flags for their use case. Below is a list of the available
feature flags and the combinations that are recommended for each use case.

- `sqlite`: Enables SQLite support.
- `postgres`: Enables PostgreSQL support.
- `mysql`: Enables MySQL support.
- `sqlx`: Enables [sqlx] support.
- `diesel`: Enables [diesel] support.

### Feature flag matrix
| Database | Query builder | Runtime |
|----------|---------------|---------|
| `sqlite` | `sqlx`        | `runtime-async-std` |
| `sqlite` | `sqlx`        | `runtime-tokio` |
| `sqlite` | `diesel`      | |
| `mysql`  | `sqlx`        | `runtime-async-std` |
| `mysql`  | `sqlx`        | `runtime-tokio` |
| `mysql`  | `diesel`      | |
| `postgres` | `sqlx`      | `runtime-async-std` |
| `postgres` | `sqlx`      | `runtime-tokio` |
| `postgres` | `diesel`    | |

### Combining feature flags

The following are the recommended feature flag combinations for each use case.

First pick one of the following database feature flags:

* `sqlite`
* `mysql`
* `postgres`

Then pick one of the following database query building feature flags:

* `sqlx`
* `diesel`

If you're using `sqlx`, you also have to pick one of the following runtime feature flags:

* `runtime-async-std`
* `runtime-tokio`

### Example

```toml
[dependencies]
database-schema = { version = "0.1", features = ["sqlite", "sqlx", "runtime-async-std"] }
```

alternatively, if you're using `diesel`:
```toml
[dependencies]
database-schema = { version = "0.1", features = ["sqlite", "diesel"] }
```

### Macros

This crate also provides a set of macros that can be used to generate the SQL
structure of a database at compile time. This is useful for generating the SQL from
`build.rs`.


```toml
[dependencies]
database-schema = { version = "0.1", features = ["sqlite", "diesel", "macros"] }
```

```rust
use database_schema::macros::generate_without_runtime;

let sql = generate_without_runtime!("./migrations", "structure.sql");
```

The above is strictly equivalent to calling:

```rust
use database_schema::macros::generate_without_runtime_using_defaults;

let sql = generate_without_runtime!();
```

## Customization

```rust
use database_schema::DatabaseSchemaBuilder;

let migrations_path = "db/migrations";
let destination_path = "db/structure.sql";

// This assumes you're using SQLite in memory.
//
// If you need to set up a `connection_url` you can use
// `DatabaseSchemaBuilder::connection_url` before calling
// `build()`.

DatabaseSchemaBuilder::new()
    .migrations_dir(migrations_path)?
    .destination_path(destination_path)
    .build()
    .dump()
    .await
```

[feature flags]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section
[sqlx]: https://docs.rs/sqlx/latest/sqlx/
[diesel]: https://docs.rs/diesel/latest/diesel/
[schema dump]: https://guides.rubyonrails.org/active_record_migrations.html#schema-dumping-and-you
