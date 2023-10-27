use crate::wasm_generator::{
    clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError, WasmGenerator,
};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct IsOk;

impl Word for IsOk {
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
            panic!("Expected a Response type. Found: {:?}", ty);
        };

        // Drop the err type.
        drop_value(builder, err_ty);

        // Drop the ok type.
        drop_value(builder, ok_ty);

        // Indicator is on stack.
        Ok(())
    }
}
