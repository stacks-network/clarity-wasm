use std::cell::OnceCell;

use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{
    types::{signatures::CallableSubtype, SequenceSubtype, StringSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};
use walrus::{ir::BinaryOp, InstrSeqBuilder, LocalId};

use super::Word;

#[derive(Debug)]
pub struct IsEq;

impl Word for IsEq {
    fn name(&self) -> ClarityName {
        "is-eq".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // Traverse the first operand pushing it onto the stack
        let first_op = args.get_expr(0)?;
        generator.traverse_expr(builder, first_op)?;

        // Save the first_op to a local to be further used.
        // This allows to use the first_op value without
        // traversing again the expression.
        let ty = generator
            .get_expr_type(first_op)
            .expect("is-eq value expression must be typed")
            .clone();
        let val_locals = generator.save_to_locals(builder, &ty, true);

        // Explicitly set to true.
        // Shortcut for a case with only one operand.
        builder.i32_const(1);

        // Loop through remainder operands, if the case.
        for operand in args.iter().skip(1) {
            // push the new operand on the stack
            generator.traverse_expr(builder, operand)?;

            // insert the new operand into locals
            let mut nth_locals = Vec::with_capacity(wasm_types.len());
            for local_ty in wasm_types.iter().rev() {
                let local = generator.module.locals.add(*local_ty);
                nth_locals.push(local);
                builder.local_set(local);
            }
            nth_locals.reverse();

            // check equality
            wasm_equal(&ty, generator, builder, &val_locals, &nth_locals)?;

            // Do an "and" operation with the result from the previous function call.
            builder.binop(BinaryOp::I32And);
        }

        Ok(())
    }
}

fn wasm_equal(
    ty: &TypeSignature,
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
) -> Result<(), GeneratorError> {
    match ty {
        // is-eq-int function can be reused to both int and uint types.
        TypeSignature::IntType | TypeSignature::UIntType => {
            wasm_equal_int128(generator, builder, first_op, nth_op)
        }
        // is-eq-bytes function can be used for types with (offset, length)
        TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
        | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_)))
        | TypeSignature::PrincipalType
        | TypeSignature::CallableType(CallableSubtype::Principal(_)) => {
            wasm_equal_bytes(generator, builder, first_op, nth_op)
        }
        _ => {
            dbg!(ty);
            Err(GeneratorError::NotImplemented)
        }
    }
}

fn wasm_equal_int128(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
) -> Result<(), GeneratorError> {
    // Get first operand from the local and put it onto stack.
    for val in first_op {
        builder.local_get(*val);
    }

    // Get second operand from the local and put it onto stack.
    for val in nth_op {
        builder.local_get(*val);
    }

    // Call the function with the operands on the stack.
    let func = OnceCell::new();
    builder.call(*func.get_or_init(|| generator.func_by_name("stdlib.is-eq-int")));

    Ok(())
}

fn wasm_equal_bytes(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
) -> Result<(), GeneratorError> {
    // Get first operand from the local and put it onto stack.
    for val in first_op {
        builder.local_get(*val);
    }

    // Get second operand from the local and put it onto stack.
    for val in nth_op {
        builder.local_get(*val);
    }

    // Call the function with the operands on the stack.
    let func = OnceCell::new();
    builder.call(*func.get_or_init(|| generator.func_by_name("stdlib.is-eq-bytes")));

    Ok(())
}
