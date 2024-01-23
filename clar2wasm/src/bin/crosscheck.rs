use std::fs;

use clap::Parser;
use clar2wasm::tools::crosscheck_compare_only;

/// crosscheck is a tool to compare the results of the compiled and interpreted
/// versions of a Clarity snippet.
#[derive(Parser)]
#[command(name = "crosscheck", version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Clarity source file to compile
    input: String,
}

fn main() {
    let args = Args::parse();

    // Require a .clar extension
    if !args.input.ends_with(".clar") {
        eprintln!("Input file must have a .clar extension");
        std::process::exit(1);
    }

    // Read the file.
    let source = match fs::read_to_string(args.input.as_str()) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error reading file: {}", error);
            std::process::exit(1);
        }
    };

    crosscheck_compare_only(&source);
}
