use clarity::vm::types::TypeSignature;
use clarity::vm::ClarityName;

use super::SimpleWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

fn simple_typed_one_call(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    _arg_types: &[TypeSignature],
    return_type: &TypeSignature,
    name: &str,
) -> Result<(), GeneratorError> {
    let type_suffix = match return_type {
        TypeSignature::IntType => "int",
        TypeSignature::UIntType => "uint",
        _ => {
            return Err(GeneratorError::TypeError(
                "invalid type for arithmetic".to_string(),
            ));
        }
    };

    let func = generator.func_by_name(&format!("stdlib.{name}-{type_suffix}"));
    builder.call(func);

    Ok(())
}

fn simple_typed_multi_value(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    n_args: usize,
    return_type: &TypeSignature,
    name: &str,
) -> Result<(), GeneratorError> {
    let type_suffix = match return_type {
        TypeSignature::IntType => "int",
        TypeSignature::UIntType => "uint",
        _ => {
            return Err(GeneratorError::TypeError(
                "invalid type for arithmetic".to_string(),
            ));
        }
    };

    let func = generator.func_by_name(&format!("stdlib.{name}-{type_suffix}"));

    // call one time less than the number of args
    for _ in 1..n_args {
        builder.call(func);
    }

    Ok(())
}

#[derive(Debug)]
pub struct Add;

impl SimpleWord for Add {
    fn name(&self) -> ClarityName {
        "+".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, arg_types.len(), return_type, "add")
    }
}

#[derive(Debug)]
pub struct Sub;

impl SimpleWord for Sub {
    fn name(&self) -> ClarityName {
        "-".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let n_args = if arg_types.len() == 1 {
            builder.i64_const(0);
            builder.i64_const(0);

            arg_types.len() + 1
        } else {
            arg_types.len()
        };
        simple_typed_multi_value(generator, builder, n_args, return_type, "sub")
    }
}

#[derive(Debug)]
pub struct Mul;

impl SimpleWord for Mul {
    fn name(&self) -> ClarityName {
        "*".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, arg_types.len(), return_type, "mul")
    }
}

#[derive(Debug)]
pub struct Div;

impl SimpleWord for Div {
    fn name(&self) -> ClarityName {
        "/".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, arg_types.len(), return_type, "div")
    }
}

#[derive(Debug)]
pub struct Modulo;

impl SimpleWord for Modulo {
    fn name(&self) -> ClarityName {
        "mod".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_one_call(generator, builder, arg_types, return_type, "mod")
    }
}

#[derive(Debug)]
pub struct Log2;

impl SimpleWord for Log2 {
    fn name(&self) -> ClarityName {
        "log2".into()
    }

    fn visit<'b>(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_one_call(generator, builder, arg_types, return_type, "log2")
    }
}

#[derive(Debug)]
pub struct Power;

impl SimpleWord for Power {
    fn name(&self) -> ClarityName {
        "pow".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_one_call(generator, builder, arg_types, return_type, "pow")
    }
}

#[derive(Debug)]
pub struct Sqrti;

impl SimpleWord for Sqrti {
    fn name(&self) -> ClarityName {
        "sqrti".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        simple_typed_one_call(generator, builder, arg_types, return_type, "sqrti")
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::{crosscheck, evaluate};

    #[test]
    fn test_overflow() {
        crosscheck("(+ u340282366920938463463374607431768211455 u1)", Err(()));
    }

    #[test]
    fn test_underflow() {
        crosscheck("(- u0 u1)", Err(()))
    }

    #[test]
    fn test_subtraction_small() {
        crosscheck("(- 1 3)", Ok(Some(Value::Int(-2))))
    }

    #[test]
    fn test_subtraction() {
        crosscheck("(- 4 3 2 1)", Ok(Some(Value::Int(-2))))
    }

    #[test]
    fn test_subtraction_unary() {
        crosscheck("(- 1)", Ok(Some(Value::Int(-1))));
        crosscheck("(- 2)", Ok(Some(Value::Int(-2))));
        crosscheck("(- 123239)", Ok(Some(Value::Int(-123239))));
    }

    #[test]
    fn test_subtraction_nullary() {
        crosscheck("(-)", Err(()));
    }

    #[test]
    fn test_subtraction_2() {
        crosscheck("(- 1 2 3 4)", Ok(Some(Value::Int(-8))))
    }

    #[test]
    fn test_add() {
        crosscheck("(+ 1 2 3)", Ok(Some(Value::Int(6))));
    }

    #[test]
    fn test_sub_three() {
        crosscheck("(- 1 2 3)", Ok(Some(Value::Int(-4))));
    }

    #[test]
    fn test_mul_three() {
        crosscheck("(* 1 2 3)", Ok(Some(Value::Int(6))));
    }

    #[test]
    fn test_div() {
        crosscheck("(/ 8 2 2)", Ok(Some(Value::Int(2))));
    }

    #[test]
    fn test_div_unary() {
        crosscheck("(/ 8)", Ok(Some(Value::Int(8))));
    }

    #[test]
    fn test_mod() {
        crosscheck("(mod 8 3)", Ok(Some(Value::Int(2))));
    }

    #[test]
    fn test_log2() {
        crosscheck("(log2 8)", Ok(Some(Value::Int(3))));
    }

    #[test]
    fn test_pow() {
        crosscheck("(pow 2 3)", Ok(Some(Value::Int(8))));
    }

    #[test]
    fn test_sqrti() {
        crosscheck("(sqrti 8)", Ok(Some(Value::Int(2))));
    }

    #[test]
    fn add() {
        crosscheck(
            "
(define-public (simple)
  (ok (+ 1 2)))
(simple)
",
            evaluate("(ok 3)"),
        );
    }

    const ARITH: &str = "
(define-public (less-uint)
    (ok (< u1 u2)))

(define-public (greater-int)
    (ok (> -1000 -2000)))

(define-public (less-or-equal-uint)
    (ok (<= u42 u42)))

(define-public (greater-or-equal-int)
    (ok (>= 42 -5130)))
";

    #[test]
    fn test_less_than() {
        crosscheck(&format!("{ARITH} (less-uint)"), evaluate("(ok true)"));
    }

    #[test]
    fn test_greater_or_equal_int() {
        crosscheck(
            &format!("{ARITH} (greater-or-equal-int)"),
            evaluate("(ok true)"),
        );
    }

    #[test]
    fn test_regress_three() {
        crosscheck(
            &format!("(* 0 5 -34028236692093846346337460743176821146)"),
            Ok(Some(Value::Int(0))),
        );
    }
}
