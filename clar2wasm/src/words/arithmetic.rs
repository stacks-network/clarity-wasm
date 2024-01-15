use clarity::vm::types::TypeSignature;
use clarity::vm::ClarityName;

use super::SimpleWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

fn simple_typed_multi_value(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    arg_types: &[TypeSignature],
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
    for _ in 1..arg_types.len() {
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "add")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "sub")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "mul")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "div")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "mod")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "log2")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "pow")
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
        simple_typed_multi_value(generator, builder, arg_types, return_type, "sqrti")
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::TestEnvironment;

    #[test]
    fn test_overflow() {
        let mut env = TestEnvironment::default();
        env.evaluate("(+ u340282366920938463463374607431768211455 u1)")
            .expect_err("should error");
    }

    #[test]
    fn test_underflow() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet("snippet", "(- u0 u1)")
            .expect_err("should error");
    }
}
