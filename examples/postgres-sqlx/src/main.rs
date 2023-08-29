use database_schema::DatabaseSchemaBuilder;

#[tokio::main]
async fn main() -> Result<(), database_schema::Error> {
    // ssl seems to not be working due to tls-native and tls-rustls shenanigans I haven't
    // figured out yet.
    //
    // In principle, it should work without code changes - only upstream changes.
    //
    // See more at https://github.com/launchbadge/sqlx/issues/1162

    DatabaseSchemaBuilder::new()
        .connection_url("postgresql://postgres@127.0.0.1:5432/postgres")
        //.connection_url("postgresql://postgres@127.0.0.1:5432/postgres?sslmode=verify-ca&sslcert=certs/client-cert.pem&sslkey=certs/client-key.pem&sslrootcert=certs/root-ca.pem")
        .migrations_dir("./migrations")?
        .destination_path("./structure.sql")
        .build()
        .dump()
        .await
}
