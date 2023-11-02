use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::super::STDLIB_PREFIX;
use super::Word;

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
        generator.traverse_args(builder, args)?;

        builder.call(generator.func_by_name("not"));

        Ok(())
    }
}
