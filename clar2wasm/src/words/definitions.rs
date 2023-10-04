use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct DefineConstant;

impl Word for DefineConstant {
    fn name(&self) -> ClarityName {
        "define-constant".into()
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
pub struct DefinePrivate;

impl Word for DefinePrivate {
    fn name(&self) -> ClarityName {
        "define-private".into()
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
pub struct DefinePublic;

impl Word for DefinePublic {
    fn name(&self) -> ClarityName {
        "define-public".into()
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
pub struct DefineReadOnly;

impl Word for DefineReadOnly {
    fn name(&self) -> ClarityName {
        "define-read-only".into()
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
pub struct DefineMap;

impl Word for DefineMap {
    fn name(&self) -> ClarityName {
        "define-map".into()
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
pub struct DefineDataVar;

impl Word for DefineDataVar {
    fn name(&self) -> ClarityName {
        "define-data-var".into()
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
pub struct DefineFungibleToken;

impl Word for DefineFungibleToken {
    fn name(&self) -> ClarityName {
        "define-fungible-token".into()
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
pub struct DefineNonFungibleToken;

impl Word for DefineNonFungibleToken {
    fn name(&self) -> ClarityName {
        "define-non-fungible-token".into()
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
