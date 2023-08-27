fn main() {
    database_schema::generate_without_runtime_using_defaults!();
    // The above is equivalent to:
    //
    // use std::path::PathBuf;
    // let migrations_path = PathBuf::from("./migrations").canonicalize().unwrap();
    // let destination_path = PathBuf::from("./structure.sql");
    // database_schema::generate_without_runtime!(migrations_path, destination_path);
}
