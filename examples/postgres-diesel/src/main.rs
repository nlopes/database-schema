use database_schema::DatabaseSchemaBuilder;

#[tokio::main]
async fn main() -> Result<(), database_schema::Error> {
    DatabaseSchemaBuilder::new()
        //.connection_url("postgresql://postgres@127.0.0.1:5432/postgres")
        .connection_url("postgresql://postgres@127.0.0.1:5432/postgres?sslmode=verify-ca&sslcert=certs/client-cert.pem&sslkey=certs/client-key.pem&sslrootcert=certs/root-ca.pem")
        .migrations_dir("./migrations")?
        .destination_path("./structure.sql")
        .build()
        .dump()
        .await
}
