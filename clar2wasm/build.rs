/// Generate the standard library as a Wasm binary from the WAT source.
#[allow(clippy::expect_used)]
fn main() {
    match wat::parse_file("src/standard/standard.wat") {
        Ok(binary) => {
            std::fs::write("src/standard/standard.wasm", binary)
                .expect("Failed to write standard library");
        }
        Err(error) => {
            panic!("Failed to parse standard library: {error}");
        }
    };
    println!("cargo:rerun-if-changed=src/standard/standard.wat");
}
