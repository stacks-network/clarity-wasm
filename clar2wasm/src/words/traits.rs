use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct DefineTrait;

impl Word for DefineTrait {
    fn name(&self) -> ClarityName {
        "define-trait".into()
    }

    fn traverse(
        &self,
        _generator: &mut WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        todo!()
    }
}

#[derive(Debug)]
pub struct UseTrait;

impl Word for UseTrait {
    fn name(&self) -> ClarityName {
        "use-trait".into()
    }

    fn traverse(
        &self,
        _generator: &mut WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        todo!()
    }
}

#[derive(Debug)]
pub struct ImplTrait;

impl Word for ImplTrait {
    fn name(&self) -> ClarityName {
        "impl-trait".into()
    }

    fn traverse(
        &self,
        _generator: &mut WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        todo!()
    }
}
