extern crate lazy_static;

use clarity::types::StacksEpochId;
use clarity::vm::analysis::{run_analysis, AnalysisDatabase, ContractAnalysis};
use clarity::vm::ast::{build_ast_with_diagnostics, ContractAST};
use clarity::vm::costs::{ExecutionCost, LimitedCostTracker};
use clarity::vm::diagnostic::Diagnostic;
use clarity::vm::types::{
    FixedFunction, ListTypeData, QualifiedContractIdentifier, SequenceSubtype, TypeSignature,
};
use clarity::vm::ClarityVersion;
pub use walrus::Module;
use wasm_generator::{GeneratorError, WasmGenerator};

mod deserialize;
pub mod initialize;
pub mod linker;
mod serialize;
pub mod wasm_generator;
pub mod wasm_utils;
mod words;

pub mod datastore;
pub mod tools;

mod error_mapping;

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
    pub ast: ContractAST,
    pub diagnostics: Vec<Diagnostic>,
    pub module: Module,
    pub contract_analysis: ContractAnalysis,
}

#[derive(Debug)]
pub enum CompileError {
    Generic {
        ast: ContractAST,
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
    let (ast, mut diagnostics, success) = build_ast_with_diagnostics(
        contract_id,
        source,
        &mut cost_tracker,
        clarity_version,
        epoch,
    );

    if !success {
        return Err(CompileError::Generic {
            ast,
            diagnostics,
            cost_tracker: Box::new(cost_tracker),
        });
    }

    // Run the analysis passes
    let mut contract_analysis = match run_analysis(
        contract_id,
        &ast.expressions,
        analysis_db,
        false,
        cost_tracker,
        epoch,
        clarity_version,
        true,
    ) {
        Ok(contract_analysis) => contract_analysis,
        Err((e, cost_track)) => {
            diagnostics.push(Diagnostic::err(&e.err));
            return Err(CompileError::Generic {
                ast,
                diagnostics,
                cost_tracker: Box::new(cost_track),
            });
        }
    };

    typechecker_workaround(&ast, &mut contract_analysis);

    // Now that the typechecker pass is done, we can concretize the expressions types which
    // might contain `ListUnionType` or `CallableType`
    #[allow(clippy::expect_used)]
    if let Err(e) = utils::concretize(&mut contract_analysis) {
        diagnostics.push(e.diagnostic);
        return Err(CompileError::Generic {
            ast: ast.clone(),
            diagnostics: diagnostics.clone(),
            cost_tracker: Box::new(
                contract_analysis
                    .cost_track
                    .take()
                    .expect("Failed to take cost tracker from contract analysis"),
            ),
        });
    }

    #[allow(clippy::expect_used)]
    match WasmGenerator::new(contract_analysis.clone()).and_then(WasmGenerator::generate) {
        Ok(module) => Ok(CompileResult {
            ast,
            diagnostics,
            module,
            contract_analysis,
        }),
        Err(e) => {
            diagnostics.push(Diagnostic::err(&e));
            Err(CompileError::Generic {
                ast,
                diagnostics,
                cost_tracker: Box::new(
                    contract_analysis
                        .cost_track
                        .take()
                        .expect("Failed to take cost tracker from contract analysis"),
                ),
            })
        }
    }
}

// Workarounds to make filter/fold work in cases where it would not otherwise. see issue #488
fn typechecker_workaround(ast: &ContractAST, contract_analysis: &mut ContractAnalysis) {
    for expr in ast.expressions.iter() {
        match expr
            .match_list()
            .and_then(|l| l.first())
            .and_then(|first| first.match_atom())
            .map(|atom| atom.as_str())
        {
            Some("filter") => {
                let Some(func_name) = expr.match_list().and_then(|l| l[1].match_atom()) else {
                    continue;
                };

                let entry_type = match contract_analysis
                    .get_private_function(func_name.as_str())
                    .or(contract_analysis.get_read_only_function_type(func_name.as_str()))
                {
                    Some(clarity::vm::types::FunctionType::Fixed(FixedFunction {
                        args, ..
                    })) => args[0].signature.clone(),
                    _ => continue,
                };
                let max_len = match contract_analysis
                    .type_map
                    .as_ref()
                    .and_then(|ty| ty.get_type(expr))
                {
                    Some(TypeSignature::SequenceType(SequenceSubtype::ListType(l))) => {
                        l.get_max_len()
                    }
                    _ => continue,
                };
                match (
                    ListTypeData::new_list(entry_type, max_len),
                    contract_analysis.type_map.as_mut(),
                ) {
                    (Ok(list_type), Some(tmap)) => {
                        tmap.overwrite_type(expr, TypeSignature::from(list_type))
                    }
                    _ => continue,
                }
            }
            Some("fold") => {
                // in the case of fold we need to override the type of the argument list

                let Some(func_expr) = expr.match_list().map(|l| &l[1]) else {
                    continue;
                };

                let Some(func_name) = func_expr.match_atom() else {
                    continue;
                };

                let return_type = match contract_analysis
                    .get_private_function(func_name.as_str())
                    .or(contract_analysis.get_read_only_function_type(func_name.as_str()))
                {
                    Some(clarity::vm::types::FunctionType::Fixed(FixedFunction {
                        args, ..
                    })) => args[0].signature.clone(),
                    _ => continue,
                };

                let Some(sequence_expr) = expr.match_list().map(|l| &l[2]) else {
                    continue;
                };

                if let Some(tmap) = contract_analysis.type_map.as_mut() {
                    let Some(seq_type) = tmap.get_type(sequence_expr) else {
                        continue;
                    };
                    let TypeSignature::SequenceType(SequenceSubtype::ListType(data)) = seq_type
                    else {
                        continue;
                    };

                    let Ok(list_data) = ListTypeData::new_list(return_type, data.get_max_len())
                    else {
                        continue;
                    };

                    tmap.overwrite_type(
                        sequence_expr,
                        TypeSignature::SequenceType(SequenceSubtype::ListType(list_data)),
                    );
                }
            }
            _ => continue,
        }
    }
}

pub fn compile_contract(contract_analysis: ContractAnalysis) -> Result<Module, GeneratorError> {
    let generator = WasmGenerator::new(contract_analysis)?;
    generator.generate()
}

mod utils {
    use clarity::vm::analysis::{CheckError, ContractAnalysis};
    use clarity::vm::errors::CheckErrors;
    use clarity::vm::types::signatures::FunctionReturnsSignature;
    use clarity::vm::types::{FixedFunction, FunctionType};

    pub fn concretize(contract_analysis: &mut ContractAnalysis) -> Result<(), CheckError> {
        // concretize Values types
        if let Some(mut typemap) = contract_analysis.type_map.take() {
            typemap.concretize()?;
            contract_analysis.type_map = Some(typemap);
        }

        // concretize private functions return types
        for fun_ty in contract_analysis.private_function_types.values_mut() {
            *fun_ty = concretize_function_return_type(fun_ty.clone())?;
        }

        // concretize public functions return types
        for fun_ty in contract_analysis.public_function_types.values_mut() {
            *fun_ty = concretize_function_return_type(fun_ty.clone())?;
        }

        // concretize read-only functions return types
        for fun_ty in contract_analysis.read_only_function_types.values_mut() {
            *fun_ty = concretize_function_return_type(fun_ty.clone())?;
        }

        Ok(())
    }

    fn concretize_function_return_type(ft: FunctionType) -> Result<FunctionType, CheckErrors> {
        match ft {
            FunctionType::Variadic(args, return_type) => {
                Ok(FunctionType::Variadic(args, return_type.concretize_deep()?))
            }
            FunctionType::Fixed(FixedFunction { args, returns }) => {
                Ok(FunctionType::Fixed(FixedFunction {
                    args,
                    returns: returns.concretize_deep()?,
                }))
            }
            FunctionType::UnionArgs(args, ret_type) => {
                Ok(FunctionType::UnionArgs(args, ret_type.concretize_deep()?))
            }
            FunctionType::Binary(arg1, arg2, FunctionReturnsSignature::Fixed(return_type)) => {
                Ok(FunctionType::Binary(
                    arg1,
                    arg2,
                    FunctionReturnsSignature::Fixed(return_type.concretize_deep()?),
                ))
            }
            ft => Ok(ft),
        }
    }
}
