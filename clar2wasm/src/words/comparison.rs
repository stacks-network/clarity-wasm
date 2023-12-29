use clarity::vm::types::{SequenceSubtype, StringSubtype, TypeSignature};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

fn traverse_comparison(
    name: &str,
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    generator.traverse_args(builder, args)?;

    let ty = generator
        .get_expr_type(&args[0])
        .expect("comparison operands must be typed");

    let type_suffix = match ty {
        TypeSignature::IntType => "int",
        TypeSignature::UIntType => "uint",
        // same function for buffer and string-ascii
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_)))
        | TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => "buff",
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
            // For `string-utf8`, comparison is done on a codepoint-by-codepoint basis.
            // Comparing two codepoints is the act of comparing them on a byte-by-byte basis.
            // Since we already have 32-bit unicode scalars, we can just compare them with buff.
            "buff"
        }
        _ => {
            return Err(GeneratorError::InternalError(
                "invalid type for comparison".to_string(),
            ))
        }
    };

    let func = generator
        .module
        .funcs
        .by_name(&format!("stdlib.{name}-{type_suffix}"))
        .unwrap_or_else(|| panic!("function not found: {name}-{type_suffix}"));

    builder.call(func);

    Ok(())
}

#[derive(Debug)]
pub struct CmpLess;

impl ComplexWord for CmpLess {
    fn name(&self) -> ClarityName {
        "<".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_comparison("lt", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct CmpLeq;

impl ComplexWord for CmpLeq {
    fn name(&self) -> ClarityName {
        "<=".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_comparison("le", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct CmpGreater;

impl ComplexWord for CmpGreater {
    fn name(&self) -> ClarityName {
        ">".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_comparison("gt", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct CmpGeq;

impl ComplexWord for CmpGeq {
    fn name(&self) -> ClarityName {
        ">=".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_comparison("ge", generator, builder, args)
    }
}
