use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ir::UnaryOp;

use super::ComplexWord;
use crate::wasm_generator::{drop_value, ArgumentsExt, GeneratorError, WasmGenerator};

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

impl ComplexWord for Begin {
    fn name(&self) -> ClarityName {
        "begin".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        generator.set_expr_type(
            args.last().ok_or_else(|| {
                GeneratorError::TypeError("begin must have at least one arg".to_string())
            })?,
            generator
                .get_expr_type(expr)
                .ok_or(GeneratorError::TypeError("begin must be typed".to_owned()))?
                .clone(),
        );
        generator.traverse_statement_list(builder, args)
    }
}

#[derive(Debug)]
pub struct UnwrapPanic;

impl ComplexWord for UnwrapPanic {
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
        // both cases, if this indicator is a 0, then we need to panic.

        // Get the type of the input expression
        let input_ty = generator
            .get_expr_type(input)
            .ok_or(GeneratorError::TypeError(
                "'unwrap-err' input expression must be typed".to_owned(),
            ))?
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
                let val_locals = generator.save_to_locals(builder, val_ty, true);

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    None,
                    |then| {
                        then.i32_const(Trap::Panic as i32).call(
                            generator
                                .module
                                .funcs
                                .by_name("stdlib.runtime-error")
                                .expect("stdlib.runtime-error not found"),
                        );
                    },
                    |_| {},
                );

                // Otherwise, push the value back onto the stack
                for val_local in val_locals {
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
                let ok_val_locals = generator.save_to_locals(builder, ok_ty, true);

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    None,
                    |then| {
                        then.i32_const(Trap::Panic as i32).call(
                            generator
                                .module
                                .funcs
                                .by_name("stdlib.runtime-error")
                                .expect("stdlib.runtime-error not found"),
                        );
                    },
                    |_| {},
                );

                // Otherwise, push the value back onto the stack
                for val_local in ok_val_locals {
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

impl ComplexWord for UnwrapErrPanic {
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
        generator.traverse_expr(builder, input)?;
        // The input must be a `response` type. It uses an i32 indicator, where
        // 0 means `err`. If this indicator is a 1, then we need to panic.

        // Get the type of the input expression
        let input_ty = generator
            .get_expr_type(input)
            .ok_or(GeneratorError::TypeError(
                "'unwrap-err-panic' input expression must be typed".to_owned(),
            ))?
            .clone();

        match &input_ty {
            TypeSignature::ResponseType(inner_types) => {
                // Ex. `(unwrap-err-panic (err 1))`, where the value type is
                // `(response uint uint)`, the stack will look like:
                // 1 -- err value
                // 0 -- ok value
                // 0 -- indicator
                // We need to get to the indicator, so we can save the err
                // value in a local, then drop the ok value, since this is not
                // needed. After that, we can check the indicator. If it's a 1,
                // we need to trigger a runtime error. If it's a 0, we just
                // push the err value back onto the stack and continue
                // execution.

                let (ok_ty, err_ty) = &**inner_types;

                // Save the err value in locals
                let err_val_locals = generator.save_to_locals(builder, err_ty, true);

                // Drop the err value
                drop_value(builder, ok_ty);

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    None,
                    |_| {},
                    |else_| {
                        else_.i32_const(Trap::Panic as i32).call(
                            generator
                                .module
                                .funcs
                                .by_name("stdlib.runtime-error")
                                .expect("stdlib.runtime-error not found"),
                        );
                    },
                );

                // Otherwise, push the value back onto the stack
                for val_local in err_val_locals {
                    builder.local_get(val_local);
                }

                Ok(())
            }
            _ => Err(GeneratorError::NotImplemented),
        }
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::errors::{Error, WasmError};
    use clarity::vm::Value;

    use crate::tools::{evaluate, TestEnvironment};

    #[test]
    fn test_unwrap_panic_some() {
        assert_eq!(evaluate("(unwrap-panic (some u1))",), Some(Value::UInt(1)));
    }

    #[test]
    fn test_unwrap_panic_none() {
        let mut env = TestEnvironment::default();
        let err = env
            .init_contract_with_snippet(
                "callee",
                r#"
(define-private (unwrap-opt (x (optional uint)))
    (unwrap-panic x)
)
(unwrap-opt none)
        "#,
            )
            .expect_err("should panic");
        matches!(err, Error::Wasm(WasmError::Runtime(_)));
    }

    #[test]
    fn test_unwrap_panic_ok() {
        assert_eq!(evaluate("(unwrap-panic (ok u2))",), Some(Value::UInt(2)));
    }

    #[test]
    fn test_unwrap_panic_err() {
        let mut env = TestEnvironment::default();
        let err = env
            .init_contract_with_snippet(
                "callee",
                r#"
(define-private (unwrap-opt (x (response uint uint)))
    (unwrap-panic x)
)
(unwrap-opt (err u42))
        "#,
            )
            .expect_err("should panic");
        matches!(err, Error::Wasm(WasmError::Runtime(_)));
    }

    #[test]
    fn test_unwrap_err_panic_err() {
        assert_eq!(
            evaluate("(unwrap-err-panic (err u1))",),
            Some(Value::UInt(1))
        );
    }

    #[test]
    fn test_unwrap_err_panic_ok() {
        let mut env = TestEnvironment::default();
        let err = env
            .init_contract_with_snippet(
                "callee",
                r#"
(define-private (unwrap-opt (x (response uint uint)))
    (unwrap-err-panic x)
)
(unwrap-opt (ok u42))
        "#,
            )
            .expect_err("should panic");
        matches!(err, Error::Wasm(WasmError::Runtime(_)));
    }

    /// Verify that the full response type is set correctly for the last
    /// expression in a `begin` block.
    #[test]
    fn begin_response_type_bug() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "snippet",
            r#"
(define-private (foo)
    (err u1)
)
(define-read-only (get-count-at-block (block uint))
    (begin
        (unwrap-err! (foo) (err u100))
        (ok u100)
    )
)
            "#,
        )
        .unwrap();
    }
}
