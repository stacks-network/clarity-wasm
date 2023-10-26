use crate::wasm_generator::{
    clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError, WasmGenerator,
};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};
use walrus::ir::{InstrSeqType, UnaryOp};

use super::Word;

/// `Trap` should match the values used in the standard library and is used to
/// indicate the reason for a runtime error from the Clarity code.
#[allow(dead_code)]
#[repr(i32)]
enum Trap {
    Overflow = 0,
    Underflow = 1,
    DivideByZero = 2,
    LogOfNumberLessThanOrEqualToZero = 3,
    ExpectedANonNegativeNumber = 4,
    Panic = 5,
}

#[derive(Debug)]
pub struct Begin;

impl Word for Begin {
    fn name(&self) -> ClarityName {
        "begin".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        generator.traverse_statement_list(builder, args)
    }
}

#[derive(Debug)]
pub struct UnwrapPanic;

impl Word for UnwrapPanic {
    fn name(&self) -> ClarityName {
        "unwrap-panic".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let input = args.get_expr(0)?;
        generator.traverse_expr(builder, input)?;
        // There must be either an `optional` or a `response` on the top of the
        // stack. Both use an i32 indicator, where 0 means `none` or `err`. In
        // both cases, if this indicator is a 0, then we need to early exit.

        // Get the type of the input expression
        let input_ty = generator
            .get_expr_type(input)
            .expect("try input expression must be typed")
            .clone();

        match &input_ty {
            TypeSignature::OptionalType(val_ty) => {
                // For the optional case, e.g. `(unwrap-panic (some 1))`, the stack
                // will look like:
                // 1 -- some value
                // 1 -- indicator
                // We need to get to the indicator, so we can pop the some value and
                // store it in a local, then check the indicator. If it's 0, we need to
                // trigger a runtime error. If it's a 1, we just push the some value
                // back onto the stack and continue execution.

                // Save the value in locals
                let wasm_types = clar2wasm_ty(val_ty);
                let mut val_locals = Vec::with_capacity(wasm_types.len());
                for local_ty in wasm_types.iter().rev() {
                    let local = generator.module.locals.add(*local_ty);
                    val_locals.push(local);
                    builder.local_set(local);
                }

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    InstrSeqType::new(&mut generator.module.types, &[], &[]),
                    |then| {
                        then.i32_const(Trap::Panic as i32).call(
                            generator
                                .module
                                .funcs
                                .by_name("runtime-error")
                                .expect("runtime_error not found"),
                        );
                    },
                    |_| {},
                );

                // Otherwise, push the value back onto the stack
                for &val_local in val_locals.iter().rev() {
                    builder.local_get(val_local);
                }

                Ok(())
            }
            TypeSignature::ResponseType(inner_types) => {
                // Ex. `(unwrap-panic (ok 1))`, where the value type is
                // `(response uint uint)`, the stack will look like:
                // 0 -- err value
                // 1 -- ok value
                // 1 -- indicator
                // We need to get to the indicator, so we can drop the err value, since
                // that is not needed, then we can pop the ok value and store them in a
                // local. Now we can check the indicator. If it's 0, we need to trigger
                // a runtime error. If it's a 1, we just push the ok value back onto
                // the stack and continue execution.

                let (ok_ty, err_ty) = &**inner_types;

                // Drop the err value
                drop_value(builder, err_ty);

                // Save the ok value in locals
                let ok_wasm_types = clar2wasm_ty(ok_ty);
                let mut ok_val_locals = Vec::with_capacity(ok_wasm_types.len());
                for local_ty in ok_wasm_types.iter().rev() {
                    let local = generator.module.locals.add(*local_ty);
                    ok_val_locals.push(local);
                    builder.local_set(local);
                }

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    InstrSeqType::new(&mut generator.module.types, &[], &[]),
                    |then| {
                        then.i32_const(Trap::Panic as i32).call(
                            generator
                                .module
                                .funcs
                                .by_name("runtime-error")
                                .expect("runtime_error not found"),
                        );
                    },
                    |_| {},
                );

                // Otherwise, push the value back onto the stack
                for &val_local in ok_val_locals.iter().rev() {
                    builder.local_get(val_local);
                }

                Ok(())
            }
            _ => Err(GeneratorError::NotImplemented),
        }
    }
}

#[derive(Debug)]
pub struct UnwrapErrPanic;

impl Word for UnwrapErrPanic {
    fn name(&self) -> ClarityName {
        "unwrap-err-panic".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let input = args.get_expr(0)?;
        let throws = args.get_expr(1)?;
        generator.traverse_expr(builder, input)?;
        generator.traverse_expr(builder, throws)
    }
}
