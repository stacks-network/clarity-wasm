use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::SimpleVariadicWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

fn simple_typed_multi_value(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    expr: &SymbolicExpression,
    n_args: usize,
    name: &str,
) -> Result<(), GeneratorError> {
    let ty = generator
        .get_expr_type(expr)
        .expect("arithmetic expression must be typed");

    let type_suffix = match ty {
        TypeSignature::IntType => "int",
        TypeSignature::UIntType => "uint",
        _ => {
            return Err(GeneratorError::InternalError(
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

impl SimpleVariadicWord for Add {
    fn name(&self) -> ClarityName {
        "+".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "add")
    }
}

#[derive(Debug)]
pub struct Sub;

impl SimpleVariadicWord for Sub {
    fn name(&self) -> ClarityName {
        "-".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "sub")
    }
}

#[derive(Debug)]
pub struct Mul;

impl SimpleVariadicWord for Mul {
    fn name(&self) -> ClarityName {
        "*".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "mul")
    }
}

#[derive(Debug)]
pub struct Div;

impl SimpleVariadicWord for Div {
    fn name(&self) -> ClarityName {
        "/".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "div")
    }
}

#[derive(Debug)]
pub struct Modulo;

impl SimpleVariadicWord for Modulo {
    fn name(&self) -> ClarityName {
        "mod".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "mod")
    }
}

#[derive(Debug)]
pub struct Log2;

impl SimpleVariadicWord for Log2 {
    fn name(&self) -> ClarityName {
        "log2".into()
    }

    fn traverse<'b>(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "log2")
    }
}

#[derive(Debug)]
pub struct Power;

impl SimpleVariadicWord for Power {
    fn name(&self) -> ClarityName {
        "pow".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "pow")
    }
}

#[derive(Debug)]
pub struct Sqrti;

impl SimpleVariadicWord for Sqrti {
    fn name(&self) -> ClarityName {
        "sqrti".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        n_args: usize,
    ) -> Result<(), GeneratorError> {
        simple_typed_multi_value(generator, builder, expr, n_args, "sqrti")
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::TestEnvironment;

    #[test]
    fn test_overflow() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "snippet",
            "(+ u340282366920938463463374607431768211455 u1)",
        )
        .expect_err("should panic");
    }

    #[test]
    fn test_underflow() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet("snippet", "(- u0 u1)")
            .expect_err("should panic");
    }
}
