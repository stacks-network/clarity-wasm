/// Generate the standard library as a Wasm binary from the WAT source.
#[allow(clippy::expect_used)]
fn main() {
    println!("cargo:rerun-if-changed=src/standard/standard.wat");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let wasm_path = std::path::Path::new(&out_dir).join("standard.wasm");

    match wat::parse_file("src/standard/standard.wat") {
        Ok(binary) => {
            std::fs::write(&wasm_path, binary)
                .expect("Failed to write standard library");

            println!("cargo:rustc-env=STANDARD_WASM_PATH={}", wasm_path.display());
        }
        Err(error) => {
            panic!("Failed to parse standard library: {error}");
        }
    };
}
