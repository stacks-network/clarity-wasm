use clap::Parser;
use clar2wasm;
use std::fs;

/// clar2wasm is a compiler for generating WebAssembly from Clarity.
#[derive(Parser)]
#[command(name = "clar2wasm", version = option_env!("CARGO_PKG_VERSION").expect("Unable to detect version"))]
struct Args {
    /// Clarity source file to compile
    input: String,
    /// Output file to write compiled WebAssembly to
    #[arg(short, long)]
    output: Option<String>,
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

    // Pass the source code to the compiler.
    let (diagnostics, result) = clar2wasm::compile(&source);
    for diagnostic in diagnostics.iter() {
        eprintln!("{diagnostic}");
    }

    let mut module = match result {
        Ok(module) => module,
        Err(_) => {
            std::process::exit(1);
        }
    };

    // Write the compiled WebAssembly to a file.
    let output = args.output.unwrap_or_else(|| {
        // Use the input file name with a .wasm extension
        let mut output = args.input.clone();

        // Strip the .clar and add .wasm
        output.replace_range(output.len() - 4.., "wasm");
        output
    });

    if let Err(error) = module.emit_wasm_file(output.as_str()) {
        eprintln!("Error writing Wasm file, {}: {}", output, error);
        std::process::exit(1);
    }
}
