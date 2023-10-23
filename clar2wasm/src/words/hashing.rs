use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{
    types::{SequenceSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};

use super::Word;

pub fn traverse_hash(
    name: &'static str,
    mem_size: usize,
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    _expr: &SymbolicExpression,
    args: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    let value = args.get_expr(0)?;
    generator.traverse_expr(builder, value)?;

    let offset_res = generator.literal_memory_end;

    generator.literal_memory_end += mem_size as u32; // 5 u32

    let ty = generator
        .get_expr_type(value)
        .expect("Hash value should be typed");
    let hash_type = match ty {
        TypeSignature::IntType | TypeSignature::UIntType => "int",
        TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => "buf",
        _ => {
            return Err(GeneratorError::NotImplemented);
        }
    };
    let hash_func = generator
        .module
        .funcs
        .by_name(&format!("{name}-{hash_type}"))
        .unwrap_or_else(|| panic!("function not found: {name}-{hash_type}"));

    builder
        .i32_const(offset_res as i32) // result offset
        .call(hash_func);

    Ok(())
}

#[derive(Debug)]
pub struct Hash160;

impl Word for Hash160 {
    fn name(&self) -> ClarityName {
        "hash160".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_hash(
            "hash160",
            core::mem::size_of::<u32>() * 5,
            generator,
            builder,
            expr,
            args,
        )
    }
}

#[derive(Debug)]
pub struct Sha256;

impl Word for Sha256 {
    fn name(&self) -> ClarityName {
        "sha256".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_hash(
            "sha256",
            core::mem::size_of::<u32>() * 8,
            generator,
            builder,
            expr,
            args,
        )
    }
}
