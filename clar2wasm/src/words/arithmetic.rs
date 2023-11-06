use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};

use super::Word;

// Wrapper function for multi-value typed functions, such as +, - etc
pub fn traverse_typed_multi_value(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    expr: &SymbolicExpression,
    args: &[SymbolicExpression],
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

    generator.traverse_expr(builder, &args[0])?;
    for operand in args.iter().skip(1) {
        generator.traverse_expr(builder, operand)?;
        builder.call(func);
    }

    Ok(())
}

#[derive(Debug)]
pub struct Add;

impl Word for Add {
    fn name(&self) -> ClarityName {
        "+".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "add")
    }
}

#[derive(Debug)]
pub struct Sub;

impl Word for Sub {
    fn name(&self) -> ClarityName {
        "-".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "sub")
    }
}

#[derive(Debug)]
pub struct Mul;

impl Word for Mul {
    fn name(&self) -> ClarityName {
        "*".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "mul")
    }
}

#[derive(Debug)]
pub struct Div;

impl Word for Div {
    fn name(&self) -> ClarityName {
        "/".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "div")
    }
}

#[derive(Debug)]
pub struct Modulo;

impl Word for Modulo {
    fn name(&self) -> ClarityName {
        "mod".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "mod")
    }
}

#[derive(Debug)]
pub struct Log2;

impl Word for Log2 {
    fn name(&self) -> ClarityName {
        "log2".into()
    }

    fn traverse<'b>(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "log2")
    }
}

#[derive(Debug)]
pub struct Power;

impl Word for Power {
    fn name(&self) -> ClarityName {
        "pow".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "pow")
    }
}

#[derive(Debug)]
pub struct Sqrti;

impl Word for Sqrti {
    fn name(&self) -> ClarityName {
        "sqrti".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_typed_multi_value(generator, builder, expr, args, "sqrti")
    }
}
