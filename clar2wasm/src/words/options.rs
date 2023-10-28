use crate::wasm_generator::{drop_value, ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};
use walrus::ir::BinaryOp;

use super::Word;

#[derive(Debug)]
pub struct IsSome;

impl Word for IsSome {
    fn name(&self) -> ClarityName {
        "is-some".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let opt = args.get_expr(0)?;
        generator.traverse_expr(builder, opt)?;
        // there is an optional type on top of the stack.

        // Get the type of the optional expression
        let ty = generator
            .get_expr_type(opt)
            .expect("input expression must be typed")
            .clone();

        let some_ty = if let TypeSignature::OptionalType(some_type) = &ty {
            &**some_type
        } else {
            panic!("Expected an Optional type. Found: {:?}", ty);
        };

        // Drop the some type.
        drop_value(builder, some_ty);

        // Indicator is on stack.
        Ok(())
    }
}

#[derive(Debug)]
pub struct IsNone;

impl Word for IsNone {
    fn name(&self) -> ClarityName {
        "is-none".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let opt = args.get_expr(0)?;
        generator.traverse_expr(builder, opt)?;
        // there is an optional type on top of the stack.

        // Get the type of the optional expression
        let ty = generator
            .get_expr_type(opt)
            .expect("input expression must be typed")
            .clone();

        let some_ty = if let TypeSignature::OptionalType(some_type) = &ty {
            &**some_type
        } else {
            panic!("Expected an Optional type. Found: {:?}", ty);
        };

        // Drop the some type.
        drop_value(builder, some_ty);

        // Push one to stack
        // and proceed with a XOR operation
        // to invert the indicator value
        builder.i32_const(1);
        builder.binop(BinaryOp::I32Xor);

        // Xor'ed indicator is on stack.
        Ok(())
    }
}
