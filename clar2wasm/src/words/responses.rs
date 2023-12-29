use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ir::BinaryOp;

use super::ComplexWord;
use crate::wasm_generator::{drop_value, ArgumentsExt, GeneratorError, WasmGenerator};

pub fn traverse_response(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    let res = args.get_expr(0)?;
    generator.traverse_expr(builder, res)?;
    // there is a response type on top of the stack.

    // Get the type of the response expression
    let ty = generator
        .get_expr_type(res)
        .expect("input expression must be typed")
        .clone();

    let (ok_ty, err_ty) = if let TypeSignature::ResponseType(types) = &ty {
        &**types
    } else {
        return Err(GeneratorError::TypeError(format!(
            "Expected a Response type. Found {:?}",
            ty
        )));
    };

    // Drop the err type.
    drop_value(builder, err_ty);

    // Drop the ok type.
    drop_value(builder, ok_ty);

    Ok(())
}

#[derive(Debug)]
pub struct IsOk;

impl ComplexWord for IsOk {
    fn name(&self) -> ClarityName {
        "is-ok".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_response(generator, builder, args)
    }
}

#[derive(Debug)]
pub struct IsErr;

impl ComplexWord for IsErr {
    fn name(&self) -> ClarityName {
        "is-err".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_response(generator, builder, args)?;

        // Add one to stack
        // and proceed with a XOR operation
        // to invert the indicator value
        builder.i32_const(1).binop(BinaryOp::I32Xor);

        // Xor'ed indicator is on stack.
        Ok(())
    }
}
