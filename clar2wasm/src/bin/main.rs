use std::fs;

use clap::Parser;
use clar2wasm::CompileError;
use clarity::types::StacksEpochId;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::MemoryBackingStore;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::ClarityVersion;

/// clar2wasm is a compiler for generating WebAssembly from Clarity.
#[derive(Parser)]
#[command(name = "clar2wasm", version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Clarity source file to compile
    input: String,
    /// Clarity version to use (1, 2 or 3)
    /// If any other value is provided, default Clarity3
    #[arg(short, long)]
    clarity_version: Option<u8>,
    /// Stacks epoch to use (10, 20, 205, 21, 22, 23, 24, 25 or 30)
    /// If any other value is provided, default to Epoch30
    #[arg(short, long)]
    epoch: Option<u8>,
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

    // Define some settings
    let contract_id = QualifiedContractIdentifier::transient();
    let clarity_version = ClarityVersion::from(ClarityVersionMapping::from(
        args.clarity_version.unwrap_or(2),
    ));
    let epoch = StacksEpochId::from(StacksEpochMapping::from(args.epoch.unwrap_or(25)));

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
        eprintln!("Error writing Wasm file, {}: {}", output, error);
        std::process::exit(1);
    }
}


#[derive(Debug)]
pub enum ClarityVersionMapping {
    Clarity1,
    Clarity2,
    Clarity3,
}

impl From<u8> for ClarityVersionMapping {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Clarity1,
            2 => Self::Clarity2,
            _ => Self::Clarity3, // Default to Clarity3 for any other value
        }
    }
}

impl From<ClarityVersionMapping> for ClarityVersion {
    fn from(version: ClarityVersionMapping) -> Self {
        match version {
            ClarityVersionMapping::Clarity1 => ClarityVersion::Clarity1,
            ClarityVersionMapping::Clarity2 => ClarityVersion::Clarity2,
            ClarityVersionMapping::Clarity3 => ClarityVersion::Clarity3,
        }
    }
}

#[derive(Debug)]
enum StacksEpochMapping {
    Epoch10,
    Epoch20,
    Epoch2_05,
    Epoch21,
    Epoch22,
    Epoch23,
    Epoch24,
    Epoch25,
    Epoch30,
}

impl From<u8> for StacksEpochMapping {
    fn from(value: u8) -> Self {
        match value {
            10 => Self::Epoch10,
            20 => Self::Epoch20,
            205 => Self::Epoch2_05,
            21 => Self::Epoch21,
            22 => Self::Epoch22,
            23 => Self::Epoch23,
            24 => Self::Epoch24,
            25 => Self::Epoch25,
            _ => Self::Epoch30, // Default to Epoch30 for any other value
        }
    }
}

impl From<StacksEpochMapping> for StacksEpochId {
    fn from(epoch: StacksEpochMapping) -> Self {
        match epoch {
            StacksEpochMapping::Epoch10 => StacksEpochId::Epoch10,
            StacksEpochMapping::Epoch20 => StacksEpochId::Epoch20,
            StacksEpochMapping::Epoch2_05 => StacksEpochId::Epoch2_05,
            StacksEpochMapping::Epoch21 => StacksEpochId::Epoch21,
            StacksEpochMapping::Epoch22 => StacksEpochId::Epoch22,
            StacksEpochMapping::Epoch23 => StacksEpochId::Epoch23,
            StacksEpochMapping::Epoch24 => StacksEpochId::Epoch24,
            StacksEpochMapping::Epoch25 => StacksEpochId::Epoch25,
            StacksEpochMapping::Epoch30 => StacksEpochId::Epoch30,
        }
    }
}