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
        _generator: &mut crate::wasm_generator::WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
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
        _generator: &mut crate::wasm_generator::WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
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
        _generator: &mut crate::wasm_generator::WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        todo!()
    }
}
