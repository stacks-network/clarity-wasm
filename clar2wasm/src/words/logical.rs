use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

fn traverse_logical(
    name: &str,
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    let func = generator
        .module
        .funcs
        .by_name(name)
        .unwrap_or_else(|| panic!("function not found: {name}"));

    for arg in args {
        generator.traverse_expr(builder, arg)?;
    }

    builder.call(func);

    Ok(())
}

#[derive(Debug)]
pub struct Not;

impl Word for Not {
    fn name(&self) -> ClarityName {
        "not".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_logical("not", generator, builder, args)
    }
}
