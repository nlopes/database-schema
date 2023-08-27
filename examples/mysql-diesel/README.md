# mysql-diesel

This simple example will use `diesel` to run the migrations found in
[`./migrations`](./migrations) and then generate a [`./structure.sql`](./structure.sql)
file with the database schema;

# Running

```shell
# this is an example in my MacOS laptop, using mysql from homebrew, adjust as you see fit!
$ docker compose up
$ export PATH=/opt/homebrew/opt/mysql-client@8.0/bin:$PATH
$ export RUSTFLAGS="-L/opt/homebrew/opt/mysql-client@8.0/lib"
$ cargo run
```

## Dependencies

- `libmysqlclient`
- `mysqldump`
- `docker compose` - if you want to run the example against a docker instance running
  `mysql` server.

If they are not in your default paths you will have to set them like so:

```shell
export PATH=/path/to/your/mysql-client/install-folder/bin:$PATH
export RUSTFLAGS="-L/path/to/your/lib-mysql-folder/lib"
```

