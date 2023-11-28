use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;
use crate::wasm_generator::{
    add_placeholder_for_type, clar2wasm_ty, ArgumentsExt, GeneratorError, WasmGenerator,
};

#[derive(Debug)]
pub struct ClaritySome;

impl Word for ClaritySome {
    fn name(&self) -> ClarityName {
        "some".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let value = args.get_expr(0)?;
        // (some <val>) is represented by an i32 1, followed by the value
        builder.i32_const(1);
        generator.traverse_expr(builder, value)
    }
}

#[derive(Debug)]
pub struct ClarityOk;

impl Word for ClarityOk {
    fn name(&self) -> ClarityName {
        "ok".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let value = args.get_expr(0)?;
        // (ok <val>) is represented by an i32 1, followed by the ok value,
        // followed by a placeholder for the err value
        builder.i32_const(1);
        generator.traverse_expr(builder, value)?;
        let ty = generator
            .get_expr_type(expr)
            .expect("ok expression must be typed");
        if let TypeSignature::ResponseType(inner_types) = ty {
            let err_types = clar2wasm_ty(&inner_types.1);
            for err_type in err_types.iter() {
                add_placeholder_for_type(builder, *err_type);
            }
        } else {
            panic!("expected response type");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ClarityErr;

impl Word for ClarityErr {
    fn name(&self) -> ClarityName {
        "err".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let value = args.get_expr(0)?;
        // (err <val>) is represented by an i32 0, followed by a placeholder
        // for the ok value, followed by the err value
        builder.i32_const(0);
        let ty = generator
            .get_expr_type(expr)
            .expect("err expression must be typed");
        if let TypeSignature::ResponseType(inner_types) = ty {
            let ok_types = clar2wasm_ty(&inner_types.0);
            for ok_type in ok_types.iter() {
                add_placeholder_for_type(builder, *ok_type);
            }
        } else {
            panic!("expected response type");
        }
        generator.traverse_expr(builder, value)
    }
}
