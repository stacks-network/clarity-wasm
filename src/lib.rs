#[macro_use]
extern crate lazy_static;

use clarity::vm::analysis::{run_analysis, AnalysisDatabase};
use clarity::vm::costs::{ExecutionCost, LimitedCostTracker};
use clarity::vm::database::ClarityBackingStore;
use clarity::vm::diagnostic::Diagnostic;
use clarity::{
    types::StacksEpochId,
    vm::{ast::build_ast_with_diagnostics, types::QualifiedContractIdentifier, ClarityVersion},
};
use walrus::Module;
use wasm_generator::WasmGenerator;

mod ast_visitor;
mod standard;
mod wasm_generator;

// FIXME: This is copied from stacks-blockchain
// Block limit in Stacks 2.1
pub const BLOCK_LIMIT_MAINNET_21: ExecutionCost = ExecutionCost {
    write_length: 15_000_000,
    write_count: 15_000,
    read_length: 100_000_000,
    read_count: 15_000,
    runtime: 5_000_000_000,
};

pub fn compile(
    source: &str,
    contract_id: &QualifiedContractIdentifier,
    mut cost_track: LimitedCostTracker,
    clarity_version: ClarityVersion,
    epoch: StacksEpochId,
    datastore: &mut dyn ClarityBackingStore,
) -> (Vec<Diagnostic>, Result<Module, ()>) {
    // Parse the contract
    let (mut ast, mut diagnostics, success) =
        build_ast_with_diagnostics(contract_id, source, &mut cost_track, clarity_version, epoch);

    if !success {
        return (diagnostics, Err(()));
    }

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(datastore);

    // Run the analysis passes
    let contract_analysis = match run_analysis(
        &contract_id,
        &mut ast.expressions,
        &mut analysis_db,
        false,
        cost_track,
        epoch,
        clarity_version,
    ) {
        Ok(contract_analysis) => contract_analysis,
        Err((e, _)) => {
            diagnostics.push(Diagnostic::err(&e.err));
            return (diagnostics, Err(()));
        }
    };

    let generator = WasmGenerator::new(contract_analysis);
    match generator.generate() {
        Ok(module) => return (diagnostics, Ok(module)),
        Err(e) => {
            diagnostics.push(Diagnostic::err(&e));
            return (diagnostics, Err(()));
        }
    };
}
