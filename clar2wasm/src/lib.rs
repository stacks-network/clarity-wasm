extern crate lazy_static;

use clarity::types::StacksEpochId;
use clarity::vm::analysis::{run_analysis, AnalysisDatabase, ContractAnalysis};
use clarity::vm::ast::build_ast_with_diagnostics;
use clarity::vm::costs::{ExecutionCost, LimitedCostTracker};
use clarity::vm::diagnostic::Diagnostic;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::ClarityVersion;
use walrus::Module;
use wasm_generator::{GeneratorError, WasmGenerator};

pub mod wasm_generator;
mod words;

#[cfg(feature = "developer-mode")]
pub mod datastore;
#[cfg(feature = "developer-mode")]
pub mod tools;

// FIXME: This is copied from stacks-blockchain
// Block limit in Stacks 2.1
pub const BLOCK_LIMIT_MAINNET_21: ExecutionCost = ExecutionCost {
    write_length: 15_000_000,
    write_count: 15_000,
    read_length: 100_000_000,
    read_count: 15_000,
    runtime: 5_000_000_000,
};

#[derive(Debug)]
pub struct CompileResult {
    pub diagnostics: Vec<Diagnostic>,
    pub module: Module,
    pub contract_analysis: ContractAnalysis,
}

#[derive(Debug)]
pub enum CompileError {
    Generic {
        diagnostics: Vec<Diagnostic>,
        cost_tracker: Box<LimitedCostTracker>,
    },
}

pub fn compile(
    source: &str,
    contract_id: &QualifiedContractIdentifier,
    mut cost_tracker: LimitedCostTracker,
    clarity_version: ClarityVersion,
    epoch: StacksEpochId,
    analysis_db: &mut AnalysisDatabase,
) -> Result<CompileResult, CompileError> {
    // Parse the contract
    let (mut ast, mut diagnostics, success) = build_ast_with_diagnostics(
        contract_id,
        source,
        &mut cost_tracker,
        clarity_version,
        epoch,
    );

    if !success {
        return Err(CompileError::Generic {
            diagnostics,
            cost_tracker: Box::new(cost_tracker),
        });
    }

    // Run the analysis passes
    let mut contract_analysis = match run_analysis(
        contract_id,
        &mut ast.expressions,
        analysis_db,
        false,
        cost_tracker,
        epoch,
        clarity_version,
    ) {
        Ok(contract_analysis) => contract_analysis,
        Err((e, cost_track)) => {
            diagnostics.push(Diagnostic::err(&e.err));
            return Err(CompileError::Generic {
                diagnostics,
                cost_tracker: Box::new(cost_track),
            });
        }
    };

    let generator = WasmGenerator::new(contract_analysis.clone());
    match generator.generate() {
        Ok(module) => Ok(CompileResult {
            diagnostics,
            module,
            contract_analysis,
        }),
        Err(e) => {
            diagnostics.push(Diagnostic::err(&e));
            Err(CompileError::Generic {
                diagnostics,
                cost_tracker: Box::new(contract_analysis.cost_track.take().unwrap()),
            })
        }
    }
}

pub fn compile_contract(contract_analysis: ContractAnalysis) -> Result<Module, GeneratorError> {
    let generator = WasmGenerator::new(contract_analysis);
    generator.generate()
}
