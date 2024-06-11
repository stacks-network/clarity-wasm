use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, FunctionKind, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct DefinePrivateFunction;

impl ComplexWord for DefinePrivateFunction {
    fn name(&self) -> ClarityName {
        "define-private".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        // Making sure name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {:?}",
                name
            )));
        }

        let body = args.get_expr(1)?;

        generator.traverse_define_function(builder, name, body, FunctionKind::Private)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefineReadonlyFunction;

impl ComplexWord for DefineReadonlyFunction {
    fn name(&self) -> ClarityName {
        "define-read-only".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        // Making sure name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {:?}",
                name
            )));
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

impl ComplexWord for DefinePublicFunction {
    fn name(&self) -> ClarityName {
        "define-public".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        // Making sure name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {:?}",
                name
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
    use clarity::types::StacksEpochId;
    use clarity::vm::Value;

    use crate::tools::{crosscheck, crosscheck_expect_failure, crosscheck_with_epoch, evaluate};

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
    fn validate_define_private_epoch() {
        // Epoch20
        crosscheck_with_epoch(
            "(define-private (index-of?) (ok u0))",
            Ok(None),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-private (index-of) (ok u0))",
            Err(()),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-private (element-at?) (ok u0))",
            Ok(None),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-private (element-at) (ok u0))",
            Err(()),
            StacksEpochId::Epoch20,
        );

        crosscheck_expect_failure("(define-private (index-of?) (ok u0))");

        crosscheck_expect_failure("(define-private (element-at?) (ok u0))");
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
            "(define-public (index-of) (ok u0))",
            Err(()),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-public (element-at?) (ok u0))",
            Ok(None),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-public (element-at) (ok u0))",
            Err(()),
            StacksEpochId::Epoch20,
        );

        crosscheck_expect_failure("(define-public (index-of?) (ok u0))");

        crosscheck_expect_failure("(define-public (element-at?) (ok u0))");
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
            "(define-read-only (index-of) (ok u0))",
            Err(()),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-read-only (element-at?) (ok u0))",
            Ok(None),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-read-only (element-at) (ok u0))",
            Err(()),
            StacksEpochId::Epoch20,
        );

        crosscheck_expect_failure("(define-read-only (index-of?) (ok u0))");

        crosscheck_expect_failure("(define-read-only (element-at?) (ok u0))");
    }
}
