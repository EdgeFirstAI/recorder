fn main() {
    include_walk::from("src/schemas/")
        .to("src/schemas.rs")
        .unwrap();
}
