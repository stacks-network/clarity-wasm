use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};
use walrus::ir::BinaryOp;

use super::{Word, STDLIB_PREFIX};

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

        // Equals expression needs to handle different types.
        let type_suffix = match ty {
            // is-eq-int function can be reused to both int and uint types.
            TypeSignature::IntType | TypeSignature::UIntType => "int",
            _ => {
                return Err(GeneratorError::NotImplemented);
            }
        };
        let func = generator.func_by_name(&format!("{STDLIB_PREFIX}.is-eq-{}", type_suffix));

        // Explicitly set to true.
        // Shortcut for a case with only one operand.
        builder.i32_const(1);

        // Loop through remainder operands, if the case.
        // First operand will be reused from a local var.
        for operand in args.iter().skip(1) {
            // Get first operand from the local and put it onto stack.
            for val in &val_locals {
                builder.local_get(*val);
            }

            // Traverse the next operand and put in onto stack.
            generator.traverse_expr(builder, operand)?;

            // Call the function with the operands on the stack.
            builder.call(func);

            // Do an "and" operation with the result from the previous function call.
            builder.binop(BinaryOp::I32And);
        }

        Ok(())
    }
}
