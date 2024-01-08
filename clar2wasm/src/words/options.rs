use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ir::BinaryOp;

use super::ComplexWord;
use crate::wasm_generator::{drop_value, ArgumentsExt, GeneratorError, WasmGenerator};

pub fn traverse_optional(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    let opt = args.get_expr(0)?;
    generator.traverse_expr(builder, opt)?;
    // there is an optional type on top of the stack.

    // Get the type of the optional expression
    let ty = generator
        .get_expr_type(opt)
        .ok_or(GeneratorError::TypeError(
            "input expression must be typed".to_owned(),
        ))?
        .clone();

    let some_ty = if let TypeSignature::OptionalType(some_type) = &ty {
        &**some_type
    } else {
        return Err(GeneratorError::TypeError(format!(
            "Expected an Optional type. Found {:?}",
            ty
        )));
    };

    // Drop the some type.
    drop_value(builder, some_ty);

    Ok(())
}

#[derive(Debug)]
pub struct IsSome;

impl ComplexWord for IsSome {
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
        traverse_optional(generator, builder, args)
    }
}

#[derive(Debug)]
pub struct IsNone;

impl ComplexWord for IsNone {
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
        traverse_optional(generator, builder, args)?;

        // Add one to stack
        // and proceed with a XOR operation
        // to invert the indicator value
        builder.i32_const(1).binop(BinaryOp::I32Xor);

        // Xor'ed indicator is on stack.
        Ok(())
    }
}
