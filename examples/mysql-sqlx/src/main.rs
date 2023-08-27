use database_schema::DatabaseSchemaBuilder;

#[tokio::main]
async fn main() -> Result<(), database_schema::Error> {
    DatabaseSchemaBuilder::new()
        .connection_url("mysql://root:@127.0.0.1:3306/example?ssl_mode=verify_ca&ssl_key=certs/client-key.pem&ssl_ca=certs/root-ca.pem&ssl_cert=certs/client-cert.pem")
        .migrations_dir("./migrations")?
        .destination_path("./structure.sql")
        .build()
        .dump()
        .await

    // You can also opt to dump all the databases by not specifying a database name
    // DatabaseSchemaBuilder::new()
    //     .connection_url("mysql://root:@127.0.0.1:3306/")
    //     .migrations_dir("./migrations")?
    //     .destination_path("./structure.sql")
    //     .build()
    //     .dump()
    //     .await
}
