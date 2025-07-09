use clarity::vm::types::{ASCIIData, CharType};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::{ComplexWord, Word};
use crate::check_args;
use crate::error_mapping::ErrorMap;
use crate::wasm_generator::{
    get_global, ArgumentsExt, FunctionKind, GeneratorError, LiteralMemoryEntry, WasmGenerator,
};
use crate::wasm_utils::{check_argument_count, ArgumentCountCheck};

#[derive(Debug)]
pub struct DefinePrivateFunction;

impl Word for DefinePrivateFunction {
    fn name(&self) -> ClarityName {
        "define-private".into()
    }
}

impl ComplexWord for DefinePrivateFunction {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        // Making sure name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {name:?}"
            )));
        }

        let body = args.get_expr(1)?;

        // Certain special contracts, such as PoX and signer contracts, invoke private functions directly within the contract.
        // Additionally, development tools may call private functions during contract testing.
        // Therefore, to support these use cases, private functions must also be exported by the WebAssembly module.
        let function_id =
            generator.traverse_define_function(builder, name, body, FunctionKind::Private)?;
        generator.module.exports.add(name.as_str(), function_id);

        Ok(())
    }
}

#[derive(Debug)]
pub struct DefineReadonlyFunction;

impl Word for DefineReadonlyFunction {
    fn name(&self) -> ClarityName {
        "define-read-only".into()
    }
}

impl ComplexWord for DefineReadonlyFunction {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        // Making sure name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {name:?}"
            )));
        }

        // Handling function name collision.
        // Detects duplicate names and generates
        // appropriate WebAssembly instructions to report the error.
        let entry = LiteralMemoryEntry::Ascii(name.to_string());
        if generator.literal_memory_offset.contains_key(&entry) {
            let (arg_name_offset, arg_name_len) =
                generator.add_clarity_string_literal(&CharType::ASCII(ASCIIData {
                    data: name.as_bytes().to_vec(),
                }))?;

            builder
                .i32_const(arg_name_offset as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-offset")?)
                .i32_const(arg_name_len as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-len")?)
                .i32_const(ErrorMap::NameAlreadyUsed as i32)
                .call(generator.func_by_name("stdlib.runtime-error"));

            return Ok(());
        }

        let body = args.get_expr(1)?;

        let function_id =
            generator.traverse_define_function(builder, name, body, FunctionKind::ReadOnly)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefinePublicFunction;

impl Word for DefinePublicFunction {
    fn name(&self) -> ClarityName {
        "define-public".into()
    }
}

impl ComplexWord for DefinePublicFunction {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        // Making sure name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {name:?}"
            )));
        }

        let body = args.get_expr(1)?;

        let function_id =
            generator.traverse_define_function(builder, name, body, FunctionKind::Public)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::errors::{CheckErrors, Error};
    use clarity::vm::Value;

    use crate::tools::{
        crosscheck, crosscheck_expect_failure, crosscheck_multi_contract, evaluate,
    };

    //
    // Module with tests that should only be executed
    // when running Clarity::V1.
    //
    #[cfg(feature = "test-clarity-v1")]
    mod clarity_v1 {
        use clarity::types::StacksEpochId;

        use crate::tools::crosscheck_with_epoch;

        #[test]
        fn validate_define_private_epoch() {
            // Epoch20
            crosscheck_with_epoch(
                "(define-private (index-of?) (ok u0))",
                Ok(None),
                StacksEpochId::Epoch20,
            );

            crosscheck_with_epoch(
                "(define-private (element-at?) (ok u0))",
                Ok(None),
                StacksEpochId::Epoch20,
            );
        }

        #[test]
        fn validate_define_public_epoch() {
            // Epoch20
            crosscheck_with_epoch(
                "(define-public (index-of?) (ok u0))",
                Ok(None),
                StacksEpochId::Epoch20,
            );

            crosscheck_with_epoch(
                "(define-public (element-at?) (ok u0))",
                Ok(None),
                StacksEpochId::Epoch20,
            );
        }

        #[test]
        fn validate_define_read_only_epoch() {
            // Epoch20
            crosscheck_with_epoch(
                "(define-read-only (index-of?) (ok u0))",
                Ok(None),
                StacksEpochId::Epoch20,
            );

            crosscheck_with_epoch(
                "(define-read-only (element-at?) (ok u0))",
                Ok(None),
                StacksEpochId::Epoch20,
            );
        }
    }

    #[test]
    fn define_private_less_than_two_args() {
        let result = evaluate("(define-private 21)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn define_private_more_than_two_args() {
        let result = evaluate("(define-private (a b c) 21 4)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn define_read_only_less_than_two_args() {
        let result = evaluate("(define-read-only 21)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn define_read_only_more_than_two_args() {
        let result = evaluate("(define-read-only (a b c) 21 4)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn define_public_less_than_two_args() {
        let result = evaluate("(define-public 21)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn define_public_more_than_two_args() {
        let result = evaluate("(define-public (a b c) 21 4)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }
    #[test]
    fn top_level_define_first() {
        crosscheck(
            "
(define-private (foo) u123456789)

(foo)
    ",
            Ok(Some(Value::UInt(123456789))),
        );
    }

    #[ignore = "see issue #316"]
    #[test]
    fn top_level_define_last() {
        crosscheck(
            "
(+ 3 4)

(define-private (foo) 42)
    ",
            Ok(None),
        );
    }

    #[test]
    fn call_private_with_args_nested() {
        crosscheck(
            "
(define-private (simple (a int) (b int))
  (+ a b)
)

(define-public (call-it)
  (ok (simple 1 2))
)

(call-it)
",
            evaluate("(ok 3)"),
        );
    }

    #[test]
    fn call_public() {
        let preamble = "
(define-public (simple)
  (ok 42))

(define-public (call-it)
  (simple))
";
        crosscheck(&format!("{preamble} (call-it)"), evaluate("(ok 42)"));
        crosscheck(&format!("{preamble} (simple)"), evaluate("(ok 42)"));
    }

    #[test]
    fn call_public_with_args() {
        let preamble = "
(define-public (simple (a int) (b int))
  (ok (+ a b)))

(define-public (call-it)
  (simple 1 2))
";
        crosscheck(&format!("{preamble} (call-it)"), evaluate("(ok 3)"));
        crosscheck(&format!("{preamble} (simple 20 22)"), evaluate("(ok 42)"));
    }

    #[test]
    fn define_public_err() {
        crosscheck(
            "(define-public (simple) (err 42)) (simple)",
            evaluate("(err 42)"),
        );
    }

    #[test]
    fn define_public_ok() {
        crosscheck(
            "(define-public (simple) (ok 42)) (simple)",
            evaluate("(ok 42)"),
        );
    }

    #[test]
    fn ret_none() {
        crosscheck(
            "
(define-public (ret-none)
  (ok none))

(ret-none)",
            evaluate("(ok none)"),
        );
    }

    #[test]
    fn private_function_with_list_union_type() {
        crosscheck(
            "(define-private (foo) (list 'S33GG8QRVWKM7AR8EFN0KZHWD5ZXPHKCWPCZ07BHE.A 'S530MSMK2C8KCDN61ZFMYKFXBHKAP6P32P4S74CJ3.a)) (foo)",
            evaluate("(list 'S33GG8QRVWKM7AR8EFN0KZHWD5ZXPHKCWPCZ07BHE.A 'S530MSMK2C8KCDN61ZFMYKFXBHKAP6P32P4S74CJ3.a)")
        );
    }

    #[test]
    fn public_function_with_list_union_type() {
        crosscheck(
            "(define-public (foo) (ok (list 'S33GG8QRVWKM7AR8EFN0KZHWD5ZXPHKCWPCZ07BHE.A 'S530MSMK2C8KCDN61ZFMYKFXBHKAP6P32P4S74CJ3.a))) (foo)",
            evaluate("(ok (list 'S33GG8QRVWKM7AR8EFN0KZHWD5ZXPHKCWPCZ07BHE.A 'S530MSMK2C8KCDN61ZFMYKFXBHKAP6P32P4S74CJ3.a))")
        );
    }

    #[test]
    fn read_only_function_with_list_union_type() {
        crosscheck(
            "(define-read-only (foo) (list 'S33GG8QRVWKM7AR8EFN0KZHWD5ZXPHKCWPCZ07BHE.A 'S530MSMK2C8KCDN61ZFMYKFXBHKAP6P32P4S74CJ3.a)) (foo)",
            evaluate("(list 'S33GG8QRVWKM7AR8EFN0KZHWD5ZXPHKCWPCZ07BHE.A 'S530MSMK2C8KCDN61ZFMYKFXBHKAP6P32P4S74CJ3.a)")
        );
    }

    #[test]
    fn validate_define_private() {
        // Reserved keyword
        crosscheck_expect_failure("(define-private (map) (ok true))");

        // Custom function name
        crosscheck("(define-private (a) (ok true))", Ok(None));

        // Custom functiona name duplicate
        crosscheck_expect_failure("(define-private (a) (ok true))(define-private (a) (ok true))");
    }

    #[test]
    fn validate_define_public() {
        // Reserved keyword
        crosscheck_expect_failure("(define-public (map) (ok true))");

        // Custom function name
        crosscheck("(define-public (a) (ok true))", Ok(None));

        // Custom functiona name duplicate
        crosscheck_expect_failure("(define-public (a) (ok true))(define-public (a) (ok true))");
    }

    #[test]
    fn validate_define_read_only() {
        // Rserved keyword
        crosscheck_expect_failure("(define-read-only (map) (ok true))");

        // Custom function name
        crosscheck("(define-read-only (a) (ok true))", Ok(None));

        // Custom function name duplicate
        crosscheck_expect_failure(
            "(define-read-only (a) (ok true))(define-read-only (a) (ok true))",
        );
    }

    #[test]
    fn reuse_arg_name() {
        let snippet = "
(define-private (foo (a int) (a int) (b int) (b int)) 1)
(define-private (bar (c int) (d int) (e int) (d int)) 1)
";
        crosscheck(
            &format!("{snippet} (foo 1 2 3 4)"),
            Err(Error::Unchecked(CheckErrors::NameAlreadyUsed(
                "a".to_string(),
            ))),
        );
        crosscheck(
            &format!("{snippet} (bar 1 2 3 4)"),
            Err(Error::Unchecked(CheckErrors::NameAlreadyUsed(
                "d".to_string(),
            ))),
        );
    }

    #[test]
    fn reuse_arg_name_contrac_call() {
        let first_contract_name = "callee".into();
        let first_snippet = "
(define-public (foo (a int) (a int) (b int) (b int)) (ok 1))
";

        let second_contract_name = "caller".into();
        let second_snippet = format!(r#"(contract-call? .{first_contract_name} foo 1 2 3 4)"#);

        crosscheck_multi_contract(
            &[
                (first_contract_name, first_snippet),
                (second_contract_name, &second_snippet),
            ],
            Err(Error::Unchecked(CheckErrors::NameAlreadyUsed(
                "a".to_string(),
            ))),
        );
    }

    #[test]
    fn define_read_only_duplicated() {
        crosscheck(
            r#"
(define-read-only (get-decimals) (ok u0))
(define-read-only (get-name) (ok "Rocket Token"))
(define-read-only (get-symbol) (ok "RKT"))
(define-read-only (get-symbol) (ok "RKT"))
(define-read-only (get-token-uri) (ok none))
            "#,
            Err(Error::Unchecked(CheckErrors::NameAlreadyUsed(
                "get-symbol".to_string(),
            ))),
        )
    }
}
