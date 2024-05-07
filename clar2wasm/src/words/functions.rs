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
        let body = args.get_expr(1)?;

        let function_id =
            generator.traverse_define_function(builder, name, body, FunctionKind::Public)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::{crosscheck, evaluate};

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
}
