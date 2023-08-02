use std::process::Command;

/// Generate the standard library as a Wasm binary from the WAT source.
/// If wat2wasm is unavailable, use the pre-built version checked into the repo.
fn main() {
    match Command::new("wat2wasm")
        .arg("src/standard/standard.wat")
        .arg("-o")
        .arg("src/standard/standard.wasm")
        .arg("--debug-names")
        .status()
    {
        Ok(status) => {
            if !status.success() {
                panic!("Failed to compile standard library");
            }
        }
        Err(error) => {
            println!(
                "Failed to compile standard library, using pre-built version: {}",
                error
            );
        }
    };
}
