mod utils;
use std::fs;

use clap::Parser;
use clar2wasm::CompileError;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::MemoryBackingStore;
use clarity::vm::types::QualifiedContractIdentifier;
use utils::{WrappedClarityVersion, WrappedEpochId};

/// clar2wasm is a compiler for generating WebAssembly from Clarity.
#[derive(Parser)]
#[command(name = "clar2wasm", version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Clarity source file to compile
    input: String,
    /// Clarity version to use (1, 2 or 3)
    #[arg(short, long)]
    clarity_version: Option<WrappedClarityVersion>,
    /// Stacks epoch to use (1.0, 2.0, 2.05, 2.1, 2.2, 2.3, 2.4, 2.5 or 3.0)
    #[arg(short, long)]
    stacks_epoch: Option<WrappedEpochId>,
    /// Output file to write compiled WebAssembly to
    #[arg(short, long)]
    output: Option<String>,
    /// Whether to emit cost-tracking code.
    #[arg(long, default_value_t = false)]
    cost_tracking: bool,
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
            eprintln!("Error reading file: {error}");
            std::process::exit(1);
        }
    };

    // Define some settings
    let contract_id = QualifiedContractIdentifier::transient();
    let clarity_version = args.clarity_version.unwrap_or_default().into();
    let epoch = args.stacks_epoch.unwrap_or_default().into();

    // Setup a datastore and cost tracker
    let mut datastore = MemoryBackingStore::new();
    let cost_track = LimitedCostTracker::new_free();

    // Pass the source code to the compiler.
    let result = clar2wasm::compile(
        &source,
        &contract_id,
        cost_track,
        clarity_version,
        epoch,
        &mut datastore.as_analysis_db(),
        args.cost_tracking,
    )
    .unwrap_or_else(|err| match err {
        CompileError::Generic {
            diagnostics,
            ast: _,
            cost_tracker: _,
        } => {
            for diagnostic in diagnostics.iter() {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    });

    let mut module = result.module;

    // Write the compiled WebAssembly to a file.
    let output = args.output.unwrap_or_else(|| {
        // Use the input file name with a .wasm extension
        let mut output = args.input.clone();

        // Strip the .clar and add .wasm
        output.replace_range(output.len() - 4.., "wasm");
        output
    });

    if let Err(error) = module.emit_wasm_file(output.as_str()) {
        eprintln!("Error writing Wasm file, {output}: {error}");
        std::process::exit(1);
    }
}
