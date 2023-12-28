fn main() {
    let s = database_schema::sqlite::fetch_structure("file:./test.db?mode=memory").unwrap();
    dbg!(s);
}
