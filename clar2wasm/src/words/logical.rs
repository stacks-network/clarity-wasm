use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct Not;

impl ComplexWord for Not {
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
        generator.traverse_args(builder, args)?;

        builder.call(generator.func_by_name("stdlib.not"));

        Ok(())
    }
}
