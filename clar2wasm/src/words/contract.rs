use clarity::vm::types::PrincipalData;
use clarity::vm::{ClarityName, SymbolicExpression, SymbolicExpressionType, Value};
use walrus::ValType;

use super::Word;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct AsContract;

impl Word for AsContract {
    fn name(&self) -> ClarityName {
        "as-contract".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
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

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let function_name = args.get_name(1)?;
        let contract_expr = args.get_expr(0)?;
        if let SymbolicExpressionType::LiteralValue(Value::Principal(PrincipalData::Contract(
            ref contract_identifier,
        ))) = contract_expr.expr
        {
            // This is a static contract call.
            // Push the contract identifier onto the stack
            // TODO(#111): These should be tracked for reuse, similar to the string literals
            let (id_offset, id_length) = generator.add_literal(&contract_identifier.clone().into());
            builder
                .i32_const(id_offset as i32)
                .i32_const(id_length as i32);
        } else {
            // This is a dynamic contract call (via a trait).
            // Traversing the expression should load the contract identifier
            // onto the stack.
            generator.traverse_expr(builder, contract_expr)?;
        }

        // shadow args
        let args = if args.len() >= 2 { &args[2..] } else { &[] };

        // Push the function name onto the stack
        let (fn_offset, fn_length) = generator.add_string_literal(function_name);
        builder
            .i32_const(fn_offset as i32)
            .i32_const(fn_length as i32);

        // Write the arguments to the call stack, to be read by the host
        let arg_offset = generator.module.locals.add(ValType::I32);
        builder
            .global_get(generator.stack_pointer)
            .local_set(arg_offset);
        let mut arg_length = 0;
        for arg in args {
            // Traverse the argument, pushing it onto the stack
            generator.traverse_expr(builder, arg)?;

            let arg_ty = generator
                .get_expr_type(arg)
                .expect("contract-call? argument must be typed")
                .clone();

            arg_length += generator.write_to_memory(builder, arg_offset, arg_length, &arg_ty);
        }

        // Push the arguments offset and length onto the data stack
        builder.local_get(arg_offset).i32_const(arg_length as i32);

        // Reserve space for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .expect("contract-call? expression must be typed")
            .clone();
        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &return_ty, true, true);

        // Push the return offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `contract_call`
        builder.call(generator.func_by_name("stdlib.contract_call"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::TestEnvironment;

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
            .init_contract_with_snippet(
                "check-value",
                "(contract-call? .contract-callee get-val)",
            )
            .expect("Failed to init contract.");
        assert_eq!(val.unwrap(), Value::Int(111));
    }
}
