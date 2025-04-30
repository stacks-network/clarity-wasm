use clarity::vm::clarity_wasm::get_type_size;
use clarity::vm::types::signatures::CallableSubtype;
use clarity::vm::types::{PrincipalData, TraitIdentifier, TypeSignature};
use clarity::vm::{ClarityName, SymbolicExpression, SymbolicExpressionType, Value};
use walrus::ir::BinaryOp;
use walrus::ValType;

use super::{ComplexWord, Word};
use crate::check_args;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use crate::wasm_utils::{check_argument_count, ArgumentCountCheck};

#[derive(Debug)]
pub struct AsContract;

impl Word for AsContract {
    fn name(&self) -> ClarityName {
        "as-contract".into()
    }
}

impl ComplexWord for AsContract {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(generator, builder, 1, args.len(), ArgumentCountCheck::Exact);

        let inner = args.get_expr(0)?;

        // Call the host interface function, `enter_as_contract`
        builder.call(generator.func_by_name("stdlib.enter_as_contract"));

        // Traverse the inner expression
        generator.traverse_expr(builder, inner)?;

        // Call the host interface function, `exit_as_contract`
        builder.call(generator.func_by_name("stdlib.exit_as_contract"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct ContractCall;

impl Word for ContractCall {
    fn name(&self) -> ClarityName {
        "contract-call?".into()
    }
}

impl ComplexWord for ContractCall {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(
            generator,
            builder,
            2,
            args.len(),
            ArgumentCountCheck::AtLeast
        );

        let function_name = args.get_name(1)?;
        let contract_expr = args.get_expr(0)?;
        if let SymbolicExpressionType::LiteralValue(Value::Principal(PrincipalData::Contract(
            ref contract_identifier,
        ))) = contract_expr.expr
        {
            // This is a static contract call.
            // Push an empty trait name first
            builder.i32_const(0).i32_const(0);
            // Push the contract identifier onto the stack
            // TODO(#111): These should be tracked for reuse, similar to the string literals
            let (id_offset, id_length) =
                generator.add_literal(&contract_identifier.clone().into())?;
            builder
                .i32_const(id_offset as i32)
                .i32_const(id_length as i32);
        } else {
            // This is a dynamic contract call (via a trait).
            // Push the trait name on the stack
            let dynamic_arg = contract_expr.match_atom().ok_or_else(|| {
                GeneratorError::TypeError(
                    "Dynamic contract-call? argument should be a name".to_owned(),
                )
            })?;
            // Check if the name is in local bindings first, then in current function arguments.
            let trait_name = generator
                .bindings
                .get_trait_name(dynamic_arg)
                .or_else(|| {
                    generator
                        .get_current_function_arg_type(dynamic_arg)
                        .and_then(|ty| match ty {
                            TypeSignature::CallableType(CallableSubtype::Trait(
                                TraitIdentifier { name, .. },
                            )) => Some(name),
                            _ => None,
                        })
                })
                .ok_or_else(|| {
                    GeneratorError::TypeError(
                        "Dynamic argument of contract-call? should be a trait".to_owned(),
                    )
                })?;

            let (offset, len) = generator.get_string_literal(trait_name).ok_or_else(|| {
                GeneratorError::TypeError(format!("Usage of an unimported trait: {trait_name}"))
            })?;
            builder.i32_const(offset as i32).i32_const(len as i32);
            // Traversing the expression should load the contract identifier
            // onto the stack.
            generator.traverse_expr(builder, contract_expr)?;
        }

        // shadow args
        let args = if args.len() >= 2 { &args[2..] } else { &[] };
        let args_ty: Vec<_> = args
            .iter()
            .map(|arg| {
                generator
                    .get_expr_type(arg)
                    .ok_or_else(|| {
                        GeneratorError::TypeError(
                            "contract-call? argument must be typed".to_owned(),
                        )
                    })
                    .cloned()
            })
            .collect::<Result<_, _>>()?;

        // Push the function name onto the stack
        let (fn_offset, fn_length) = generator.add_string_literal(function_name)?;
        builder
            .i32_const(fn_offset as i32)
            .i32_const(fn_length as i32);

        // Write the arguments to the call stack, to be read by the host
        let arg_offset = generator.module.locals.add(ValType::I32);
        let total_args_size = args_ty.iter().map(get_type_size).sum();
        builder
            .global_get(generator.stack_pointer)
            .local_tee(arg_offset)
            .i32_const(total_args_size)
            .binop(BinaryOp::I32Add)
            .global_set(generator.stack_pointer);

        let mut arg_length = 0;
        for (arg, arg_ty) in args.iter().zip(args_ty) {
            // Traverse the argument, pushing it onto the stack
            generator.traverse_expr(builder, arg)?;

            arg_length += generator.write_to_memory(builder, arg_offset, arg_length, &arg_ty)?;
        }

        // Push the arguments offset and length onto the data stack
        builder.local_get(arg_offset).i32_const(arg_length as i32);

        // Reserve space for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| {
                GeneratorError::TypeError("contract-call? expression must be typed".to_owned())
            })?
            .clone();
        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &return_ty, true, true);

        // Push the return offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `contract_call`
        builder.call(generator.func_by_name("stdlib.contract_call"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::{evaluate, TestEnvironment};

    #[test]
    fn as_contract_less_than_one_arg() {
        let result = evaluate("(as-contract)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 0"));
    }

    #[test]
    fn as_contract_more_than_one_arg() {
        let result = evaluate("(as-contract 1 2)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 2"));
    }

    #[test]
    fn contract_call_less_than_two_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-public (no-args)
    (ok u42)
)
            "#,
        )
        .expect("Failed to init contract.");
        let result =
            env.init_contract_with_snippet("contract-caller", "(contract-call? .contract-callee)");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting >= 2 arguments, got 1"));
    }

    #[test]
    fn static_no_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-public (no-args)
    (ok u42)
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                "(contract-call? .contract-callee no-args)",
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::UInt(42)).unwrap());
    }

    #[test]
    fn static_one_simple_arg() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-public (one-simple-arg (x int))
    (ok x)
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                "(contract-call? .contract-callee one-simple-arg 42)",
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::Int(42)).unwrap());
    }

    #[test]
    fn static_one_arg() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-public (one-arg (x (string-ascii 16)))
    (ok x)
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"(contract-call? .contract-callee one-arg "hello")"#,
            )
            .expect("Failed to init contract.");

        assert_eq!(
            val.unwrap(),
            Value::okay(Value::string_ascii_from_bytes("hello".to_string().into_bytes()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn static_two_simple_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-public (two-simple-args (x int) (y int))
    (ok (+ x y))
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"(contract-call? .contract-callee two-simple-args 17 42)"#,
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::Int(17 + 42)).unwrap());
    }

    #[test]
    fn static_two_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-public (two-args (x (string-ascii 16)) (y (string-ascii 16)))
    (ok (concat x y))
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"(contract-call? .contract-callee two-args "hello " "world")"#,
            )
            .expect("Failed to init contract.");

        assert_eq!(
            val.unwrap(),
            Value::okay(
                Value::string_ascii_from_bytes("hello world".to_string().into_bytes()).unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn dynamic_no_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-trait test-trait ((no-args () (response uint uint))))
(define-public (no-args)
    (ok u42)
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"
(use-trait test-trait .contract-callee.test-trait)
(define-private (call-it (t <test-trait>))
    (contract-call? t no-args)
)
(call-it .contract-callee)
            "#,
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::UInt(42)).unwrap());
    }

    #[test]
    fn dynamic_one_simple_arg() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-trait test-trait ((one-simple-arg (int) (response int uint))))
(define-public (one-simple-arg (x int))
    (ok x)
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"
(use-trait test-trait .contract-callee.test-trait)
(define-private (call-it (t <test-trait>) (x int))
    (contract-call? t one-simple-arg x)
)
(call-it .contract-callee 42)
            "#,
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::Int(42)).unwrap());
    }

    #[test]
    fn dynamic_one_arg() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-trait test-trait ((one-arg ((string-ascii 16)) (response (string-ascii 16) uint))))
(define-public (one-arg (x (string-ascii 16)))
    (ok x)
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"
(use-trait test-trait .contract-callee.test-trait)
(define-private (call-it (t <test-trait>) (x (string-ascii 16)))
    (contract-call? t one-arg x)
)
(call-it .contract-callee "hello")
            "#,
            )
            .expect("Failed to init contract.");

        assert_eq!(
            val.unwrap(),
            Value::okay(Value::string_ascii_from_bytes("hello".to_string().into_bytes()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn dynamic_two_simple_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-trait test-trait ((two-simple-args (int int) (response int uint))))
(define-public (two-simple-args (x int) (y int))
    (ok (+ x y))
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"
(use-trait test-trait .contract-callee.test-trait)
(define-private (call-it (t <test-trait>) (x int) (y int))
    (contract-call? t two-simple-args x y)
)
(call-it .contract-callee 17 42)
            "#,
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::Int(17 + 42)).unwrap());
    }

    #[test]
    fn dynamic_two_args() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-trait test-trait ((two-args ((string-ascii 16) (string-ascii 16)) (response (string-ascii 32) uint))))
(define-public (two-args (x (string-ascii 16)) (y (string-ascii 16)))
    (ok (concat x y))
)
            "#,
        ).expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "contract-caller",
                r#"
(use-trait test-trait .contract-callee.test-trait)
(define-private (call-it (t <test-trait>) (x (string-ascii 16)) (y (string-ascii 16)))
    (contract-call? t two-args x y)
)
(call-it .contract-callee "hello " "world")
            "#,
            )
            .expect("Failed to init contract.");

        assert_eq!(
            val.unwrap(),
            Value::okay(
                Value::string_ascii_from_bytes("hello world".to_string().into_bytes()).unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    /// Call the erroring function directly and verify that the changes are
    /// rolled back.
    fn err_rollback_direct() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-data-var my-val int 111)
(define-public (set-err (val int))
    (begin
        (var-set my-val val)
        (err u1)
    )
)
(define-read-only (get-val)
    (var-get my-val)
)
            "#,
        )
        .expect("Failed to init contract.");

        // Expect this call to return an error
        let res = env
            .init_contract_with_snippet(
                "contract-caller",
                "(contract-call? .contract-callee set-err -42)",
            )
            .expect("Failed to init contract.");
        assert_eq!(res.unwrap(), Value::err_uint(1));

        // Expect the data-var to be unchanged
        let val = env
            .init_contract_with_snippet("check-value", "(contract-call? .contract-callee get-val)")
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(111));
    }

    #[test]
    /// Call the erroring function indirectly, through another contract's
    /// function which also fails, and verify that the changes are rolled back.
    fn err_rollback() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-data-var my-val int 111)
(define-public (set-err (val int))
    (begin
        (var-set my-val val)
        (err u1)
    )
)
(define-read-only (get-val)
    (var-get my-val)
)
            "#,
        )
        .expect("Failed to init contract.");

        env.init_contract_with_snippet(
            "contract-caller",
            r#"
(define-public (call-set-err)
    (contract-call? .contract-callee set-err -42)
)
              "#,
        )
        .expect("Failed to init contract.");

        // Expect this call to return an err
        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-caller call-set-err)")
            .expect("Failed to init contract.");
        assert_eq!(res.unwrap(), Value::err_uint(1));

        // Expect the data-var to be unchanged
        let val = env
            .init_contract_with_snippet("check-value", "(contract-call? .contract-callee get-val)")
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(111));
    }

    #[test]
    /// Call the erroring function indirectly, through another contract's
    /// function which returns ok, but verify that the erroring functions'
    /// changes are still rolled back.
    fn err_rollback_ok() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-data-var my-val int 111)
(define-public (set-err (val int))
    (begin
        (var-set my-val val)
        (err u1)
    )
)
(define-read-only (get-val)
    (var-get my-val)
)
            "#,
        )
        .expect("Failed to init contract.");

        env.init_contract_with_snippet(
            "contract-caller",
            r#"
(define-public (call-set-err-ok)
    (ok (unwrap-err-panic (contract-call? .contract-callee set-err -42)))
)
              "#,
        )
        .expect("Failed to init contract.");

        // Expect this call to return an okay.
        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-caller call-set-err-ok)",
            )
            .expect("Failed to init contract.");
        assert_eq!(res.unwrap(), Value::okay(Value::UInt(1)).unwrap());

        // Expect the data-var to be unchanged
        let val = env
            .init_contract_with_snippet("check-value", "(contract-call? .contract-callee get-val)")
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(111));
    }

    #[test]
    /// Call the erroring function indirectly, through another contract's
    /// function which returns ok, but verify that the erroring functions'
    /// changes are still rolled back, while the ok function's changes are
    /// preserved.
    fn err_rollback_ok_preserve_changes() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-data-var my-val int 111)
(define-public (set-err (val int))
    (begin
        (var-set my-val val)
        (err u1)
    )
)
(define-read-only (get-val)
    (var-get my-val)
)
            "#,
        )
        .expect("Failed to init contract.");

        env.init_contract_with_snippet(
            "contract-caller",
            r#"
(define-data-var my-val int 3)
(define-public (call-set-err-ok)
    (begin
        (var-set my-val 123)
        (ok (unwrap-err-panic (contract-call? .contract-callee set-err -42)))
    )
)
(define-read-only (get-val)
    (var-get my-val)
)
              "#,
        )
        .expect("Failed to init contract.");

        // Expect this call to return an okay.
        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-caller call-set-err-ok)",
            )
            .expect("Failed to init contract.");
        assert_eq!(res.unwrap(), Value::okay(Value::UInt(1)).unwrap());

        // Expect the callee data-var to be unchanged
        let val = env
            .init_contract_with_snippet("check-value", "(contract-call? .contract-callee get-val)")
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(111));

        // Expect the caller data-var to be changed.
        let val = env
            .init_contract_with_snippet(
                "check-value-2",
                "(contract-call? .contract-caller get-val)",
            )
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(123));
    }

    #[test]
    /// Call the erroring function via an intra-contract function call (not
    /// using `contract-call?`), and verify that the changes are rolled back.
    fn err_rollback_intra_contract_call() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-data-var my-val int 111)
(define-public (set-err (val int))
    (begin
        (var-set my-val val)
        (err u1)
    )
)
(define-public (set-it)
    (ok (unwrap-err-panic (set-err -123)))
)
(define-read-only (get-val)
    (var-get my-val)
)
            "#,
        )
        .expect("Failed to init contract.");

        // Expect this call to return an okay.
        let res = env
            .init_contract_with_snippet(
                "contract-caller",
                "(contract-call? .contract-callee set-it)",
            )
            .expect("Failed to init contract.");
        assert_eq!(res.unwrap(), Value::okay(Value::UInt(1)).unwrap());

        // Expect the data-var to be unchanged
        let val = env
            .init_contract_with_snippet("check-value", "(contract-call? .contract-callee get-val)")
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(111));
    }

    #[test]
    /// Call the erroring function via an intra-contract function call (not
    /// using `contract-call?`), and verify that the changes are rolled back
    /// because the erroring function is private.
    fn err_no_rollback_intra_contract_call() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
(define-data-var my-val int 111)
(define-private (set-err (val int))
    (begin
        (var-set my-val val)
        (err u1)
    )
)
(define-public (set-it)
    (ok (unwrap-err-panic (set-err -123)))
)
(define-read-only (get-val)
    (var-get my-val)
)
            "#,
        )
        .expect("Failed to init contract.");

        // Expect this call to return an okay.
        let res = env
            .init_contract_with_snippet(
                "contract-caller",
                "(contract-call? .contract-callee set-it)",
            )
            .expect("Failed to init contract.");
        assert_eq!(res.unwrap(), Value::okay(Value::UInt(1)).unwrap());

        // Expect the data-var to be unchanged
        let val = env
            .init_contract_with_snippet("check-value", "(contract-call? .contract-callee get-val)")
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(-123));
    }

    #[test]
    fn test_nested_list_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (list-arg (lst (list 3 (list 3 int))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee list-arg (list (list 1 2 3)))",
            )
            .expect("Failed to init contract.");
        let interpreted =
            env.evaluate("(contract-call? .contract-callee list-arg (list (list 1 2 3)))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_response_in_list_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (list-arg (lst (list 3 (response int int))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee list-arg (list (ok 1) (err 2) (ok 3)))",
            )
            .expect("Failed to init contract.");
        let interpreted =
            env.evaluate("(contract-call? .contract-callee list-arg (list (ok 1) (err 2) (ok 3)))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_optional_in_list_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (list-arg (lst (list 3 (optional int))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee list-arg (list (some 1) none (some 3)))",
            )
            .expect("Failed to init contract.");
        let interpreted = env
            .evaluate("(contract-call? .contract-callee list-arg (list (some 1) none (some 3)))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_tuple_in_list_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (list-arg (lst (list 3 (tuple (a int) (b int)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee list-arg (list (tuple (a 1) (b 2)) (tuple (a 3) (b 4)) (tuple (a 5) (b 6))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee list-arg (list (tuple (a 1) (b 2)) (tuple (a 3) (b 4)) (tuple (a 5) (b 6))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_list_in_response_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (list-arg (arg (response (list 3 int) int )))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee list-arg (ok (list 1 2 3)))",
            )
            .expect("Failed to init contract.");
        let interpreted =
            env.evaluate("(contract-call? .contract-callee list-arg (ok (list 1 2 3)))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_list_in_tuple_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (tuple (a int) (b (list 3 int)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (list 2 3 4))))",
            )
            .expect("Failed to init contract.");
        let interpreted = env
            .evaluate("(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (list 2 3 4))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_list_in_response_in_tuple_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (tuple (a int) (b (response (list 3 int) int)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (ok (list 2 3 4)))))",
            )
            .expect("Failed to init contract.");
        let interpreted = env.evaluate(
            "(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (ok (list 2 3 4)))))",
        );
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_list_in_optional_in_tuple_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (tuple (a int) (b (optional (list 3 int))))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (some (list 2 3 4)))))",
            )
            .expect("Failed to init contract.");
        let interpreted = env.evaluate(
            "(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (some (list 2 3 4)))))",
        );
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_tuple_in_response_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (response (tuple (a int) (b int)) int)))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee tuple-arg (ok (tuple (a 1) (b 2))))",
            )
            .expect("Failed to init contract.");
        let interpreted =
            env.evaluate("(contract-call? .contract-callee tuple-arg (ok (tuple (a 1) (b 2))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_optional_in_response_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (opt-arg (arg (response (optional int) int)))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee opt-arg (ok (some 42)))",
            )
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee opt-arg (ok (some 42)))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_list_in_optional_in_response_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (list-arg (arg (response (optional (list 3 int)) int)))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee list-arg (ok (some (list 1 2 3))))",
            )
            .expect("Failed to init contract.");
        let interpreted =
            env.evaluate("(contract-call? .contract-callee list-arg (ok (some (list 1 2 3))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_list_in_tuple_in_response_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (response (tuple (a int) (b (list 3 int))) int)))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee tuple-arg (ok (tuple (a 1) (b (list 2 3 4)))))",
            )
            .expect("Failed to init contract.");
        let interpreted = env.evaluate(
            "(contract-call? .contract-callee tuple-arg (ok (tuple (a 1) (b (list 2 3 4)))))",
        );
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_nested_list_in_optional_in_tuple_in_response_as_argument() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (response (tuple (a int) (b (optional (list 3 int)))) int)))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee tuple-arg (ok (tuple (a 1) (b (some (list 2 3 4))))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee tuple-arg (ok (tuple (a 1) (b (some (list 2 3 4))))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_int_type_with_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (int-arg (arg (tuple (a int) (b (list 3 int)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee int-arg (tuple (a 42) (b (list 1 2 3))))",
            )
            .expect("Failed to init contract.");
        let interpreted = env
            .evaluate("(contract-call? .contract-callee int-arg (tuple (a 42) (b (list 1 2 3))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_uint_type_with_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (uint-arg (arg (tuple (a uint) (b (list 3 uint)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet(
                "call-it",
                "(contract-call? .contract-callee uint-arg (tuple (a u42) (b (list u1 u2 u3))))",
            )
            .expect("Failed to init contract.");
        let interpreted = env.evaluate(
            "(contract-call? .contract-callee uint-arg (tuple (a u42) (b (list u1 u2 u3))))",
        );
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_bool_type_with_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (bool-arg (arg (tuple (a bool) (b (list 3 bool)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee bool-arg (tuple (a true) (b (list true false true))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee bool-arg (tuple (a true) (b (list true false true))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_string_type_with_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (string-arg (arg (tuple (a (string-ascii 10)) (b (list 3 (string-ascii 10))))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee string-arg (tuple (a \"hello\") (b (list \"one\" \"two\" \"three\"))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee string-arg (tuple (a \"hello\") (b (list \"one\" \"two\" \"three\"))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_principal_type_with_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (principal-arg (arg (tuple (a principal) (b (list 3 principal)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee principal-arg (tuple (a 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY) (b (list 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee principal-arg (tuple (a 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY) (b (list 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_tuple_type_with_nested_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (tuple-arg (arg (tuple (a int) (b (list 3 (tuple (x int) (y (list 2 int))))))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (list (tuple (x 2) (y (list 3 4))) (tuple (x 5) (y (list 6 7))) (tuple (x 8) (y (list 9 10)))))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee tuple-arg (tuple (a 1) (b (list (tuple (x 2) (y (list 3 4))) (tuple (x 5) (y (list 6 7))) (tuple (x 8) (y (list 9 10)))))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_optional_type_with_nested_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (optional-arg (arg (tuple (a int) (b (optional (list 3 (tuple (x int) (y (list 2 int)))))))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee optional-arg (tuple (a 1) (b (some (list (tuple (x 2) (y (list 3 4))) (tuple (x 5) (y (list 6 7))) (tuple (x 8) (y (list 9 10))))))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee optional-arg (tuple (a 1) (b (some (list (tuple (x 2) (y (list 3 4))) (tuple (x 5) (y (list 6 7))) (tuple (x 8) (y (list 9 10))))))))");
        assert_eq!(interpreted.unwrap(), res);
    }

    #[test]
    fn test_response_type_with_nested_list() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "contract-callee",
            r#"
            (define-public (response-arg (arg (tuple (a int) (b (response (list 3 (tuple (x int) (y (list 2 int)))) int)))))
                (ok 0)
            )
            "#,
        )
        .expect("Failed to init contract.");

        let res = env
            .init_contract_with_snippet("call-it", "(contract-call? .contract-callee response-arg (tuple (a 1) (b (ok (list (tuple (x 2) (y (list 3 4))) (tuple (x 5) (y (list 6 7))) (tuple (x 8) (y (list 9 10))))))))")
            .expect("Failed to init contract.");
        let interpreted = env.evaluate("(contract-call? .contract-callee response-arg (tuple (a 1) (b (ok (list (tuple (x 2) (y (list 3 4))) (tuple (x 5) (y (list 6 7))) (tuple (x 8) (y (list 9 10))))))))");
        assert_eq!(interpreted.unwrap(), res);
    }
}
