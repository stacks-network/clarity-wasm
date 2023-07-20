use clap::Parser;
use clar2wasm;
use std::fs;

/// clar2wasm is a compiler for generating WebAssembly from Clarity.
#[derive(Parser)]
#[command(name = "clar2wasm", version = option_env!("CARGO_PKG_VERSION").expect("Unable to detect version"))]
struct Args {
    /// Clarity source file to compile
    input: String,
}

fn main() {
    let args = Args::parse();

    // Read the file.
    let source = match fs::read_to_string(args.input.as_str()) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Error reading file: {}", error);
            std::process::exit(1);
        }
    };

    // let mut datastore = MemoryBackingStore::new();
    // let mut analysis_db = AnalysisDatabase::new(&mut datastore);
    // let mut headers_db = NullHeadersDB {};
    // let mut burn_state_db = NullBurnStateDB { epoch };
    // let mut clarity_db = ClarityDatabase::new(&mut datastore, &headers_db, &burn_state_db);

    // // Create a cost tracker
    // let mut cost_track = match LimitedCostTracker::new(
    //     true,
    //     CHAIN_ID_MAINNET,
    //     BLOCK_LIMIT_MAINNET_21,
    //     &mut clarity_db,
    //     epoch,
    // ) {
    //     Ok(cost_track) => cost_track,
    //     Err(e) => {
    //         return (
    //             vec![Diagnostic {
    //                 level: clarity::vm::diagnostic::Level::Error,
    //                 message: "error creating cost tracker".to_string(),
    //                 spans: Vec::new(),
    //                 suggestion: None,
    //             }],
    //             Err(()),
    //         );
    //     }
    // };

    // Pass the source code to the compiler.
    let (diagnostics, result) = clar2wasm::compile(&source);
    for diagnostic in diagnostics.iter() {
        eprintln!("{diagnostic}");
    }
    let _bytecode = match result {
        Ok(bytecode) => bytecode,
        Err(_) => {
            std::process::exit(1);
        }
    };

    // TODO: Do something with the bytecode.
}
