use clarity::vm::representations::Span;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, FunctionKind, GeneratorError, WasmGenerator};

#[derive(Clone)]
pub struct TypedVar<'a> {
    pub name: &'a ClarityName,
    pub type_expr: &'a SymbolicExpression,
    pub decl_span: Span,
}

#[derive(Debug)]
pub struct DefinePrivateFunction;

impl ComplexWord for DefinePrivateFunction {
    fn name(&self) -> ClarityName {
        "define-private".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        let body = args.get_expr(1)?;

        generator.traverse_define_function(builder, name, body, FunctionKind::Private)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefineReadonlyFunction;

impl ComplexWord for DefineReadonlyFunction {
    fn name(&self) -> ClarityName {
        "define-read-only".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        let body = args.get_expr(1)?;

        let function_id =
            generator.traverse_define_function(builder, name, body, FunctionKind::ReadOnly)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefinePublicFunction;

impl ComplexWord for DefinePublicFunction {
    fn name(&self) -> ClarityName {
        "define-public".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        let body = args.get_expr(1)?;

        let function_id =
            generator.traverse_define_function(builder, name, body, FunctionKind::Public)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}
