# postgres-sqlx

This simple example will use `sqlx` to run the migrations found in
[`./migrations`](./migrations) and then generate a [`./structure.sql`](./structure.sql)
file with the database schema;

# Running

```shell
# this is an example in my MacOS laptop, using postgres from homebrew, adjust as you see fit!
$ docker compose up
$ export PATH=/opt/homebrew/opt/postgres-client@8.0/bin:$PATH
$ export RUSTFLAGS="-L/opt/homebrew/opt/postgres-client@8.0/lib"
$ cargo run
```

## Dependencies

- `TODO`
- `pg_dump`
- `docker compose` - if you want to run the example against a docker instance running
  `postgres` server.

If they are not in your default paths you will have to set them like so:

```shell
export PATH=/path/to/your/postgres-client/install-folder/bin:$PATH
export RUSTFLAGS="-L/path/to/your/lib-postgres-folder/lib"
```

