use crate::wasm_generator::GeneratorError;
use clarity::vm::{
    types::{SequenceSubtype, StringSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};

use super::Word;

fn traverse_comparison(
    name: &str,
    generator: &mut crate::wasm_generator::WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[clarity::vm::SymbolicExpression],
) -> Result<(), GeneratorError> {
    generator.traverse_args(builder, args)?;

    let ty = generator
        .get_expr_type(&args[0])
        .expect("comparison operands must be typed");

    let type_suffix = match ty {
        TypeSignature::IntType => "int",
        TypeSignature::UIntType => "uint",
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
            "string-ascii"
        }
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
            "string-utf8"
        }
        TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => "buffer",
        _ => {
            return Err(GeneratorError::InternalError(
                "invalid type for comparison".to_string(),
            ))
        }
    };

    let func = generator
        .module
        .funcs
        .by_name(&format!("{name}-{type_suffix}"))
        .unwrap_or_else(|| panic!("function not found: {name}-{type_suffix}"));

    builder.call(func);

    Ok(())
}

#[derive(Debug)]
pub struct CmpLess;

impl Word for CmpLess {
    fn name(&self) -> ClarityName {
        "<".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_comparison("lt", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct CmpLeq;

impl Word for CmpLeq {
    fn name(&self) -> ClarityName {
        "<=".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_comparison("le", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct CmpGreater;

impl Word for CmpGreater {
    fn name(&self) -> ClarityName {
        ">".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_comparison("gt", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct CmpGeq;

impl Word for CmpGeq {
    fn name(&self) -> ClarityName {
        ">=".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_comparison("ge", generator, builder, args)
    }
}
