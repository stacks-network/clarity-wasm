use crate::wasm_generator::{ArgumentsExt, GeneratorError};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};
use walrus::InstrSeqBuilder;

use super::Word;

#[derive(Debug)]
pub struct BitwiseNot;

impl Word for BitwiseNot {
    fn name(&self) -> ClarityName {
        "bit-not".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_expr(builder, args.get_expr(0)?)?;

        let helper_func = generator.func_by_name("bit-not");
        builder.call(helper_func);
        Ok(())
    }
}

// multi value bit-operations

fn traverse_bitwise(
    name: &'static str,
    generator: &mut crate::wasm_generator::WasmGenerator,
    builder: &mut InstrSeqBuilder,
    operands: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    let helper_func = generator.func_by_name(name);

    // Start off with operand 0, then loop over the rest, calling the
    // helper function with a pair of operands, either operand 0 and 1, or
    // the result of the previous call and the next operand.
    // e.g. (+ 1 2 3 4) becomes (+ (+ (+ 1 2) 3) 4)
    generator.traverse_expr(builder, &operands[0])?;
    for operand in operands.iter().skip(1) {
        generator.traverse_expr(builder, operand)?;
        builder.call(helper_func);
    }

    Ok(())
}

#[derive(Debug)]
pub struct BitwiseOr;

impl Word for BitwiseOr {
    fn name(&self) -> ClarityName {
        "bit-or".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_bitwise("bit-or", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct BitwiseAnd;

impl Word for BitwiseAnd {
    fn name(&self) -> ClarityName {
        "bit-and".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_bitwise("bit-and", generator, builder, args)
    }
}

#[derive(Debug)]
pub struct BitwiseXor;

impl Word for BitwiseXor {
    fn name(&self) -> ClarityName {
        "bit-xor".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_bitwise("bit-xor", generator, builder, args)
    }
}

// fn traverse_bit_shift(
//     generator: &mut crate::wasm_generator::WasmGenerator,
//     builder: &mut InstrSeqBuilder,
//     _expr: &SymbolicExpression,
//     func: NativeFunctions,
//     input: &SymbolicExpression,
//     _shamt: &SymbolicExpression,
// ) -> Result<(), GeneratorError> {

// }

#[derive(Debug)]
pub struct BitwiseLShift;

impl Word for BitwiseLShift {
    fn name(&self) -> ClarityName {
        "bit-shift-left".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let input = args.get_expr(0)?;
        let shamt = args.get_expr(1)?;

        generator.traverse_expr(builder, input)?;
        generator.traverse_expr(builder, shamt)?;
        let func = generator.func_by_name("bit-shift-left");
        builder.call(func);
        Ok(())
    }
}

#[derive(Debug)]
pub struct BitwiseRShift;

impl Word for BitwiseRShift {
    fn name(&self) -> ClarityName {
        "bit-shift-right".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let input = args.get_expr(0)?;
        let shamt = args.get_expr(1)?;

        generator.traverse_expr(builder, input)?;
        generator.traverse_expr(builder, shamt)?;

        let ty = generator
            .get_expr_type(input)
            .expect("bit shift operands must be typed");
        let type_suffix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                return Err(GeneratorError::InternalError(
                    "invalid type for shift".to_string(),
                ));
            }
        };

        let helper = generator.func_by_name(&format!("bit-shift-right-{type_suffix}"));

        builder.call(helper);

        Ok(())
    }
}
