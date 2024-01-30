use clarity::vm::types::TypeSignature;
use clarity::vm::ClarityName;
use walrus::InstrSeqBuilder;

use super::SimpleWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct BitwiseNot;

impl SimpleWord for BitwiseNot {
    fn name(&self) -> ClarityName {
        "bit-not".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let helper_func = generator.func_by_name("stdlib.bit-not");
        builder.call(helper_func);
        Ok(())
    }
}

// multi value bit-operations

fn traverse_bitwise(
    name: &'static str,
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    arg_types: &[TypeSignature],
) -> Result<(), GeneratorError> {
    let helper_func = generator.func_by_name(&format!("stdlib.{name}"));
    // Run this once for every arg except first
    for _ in arg_types.iter().skip(1) {
        builder.call(helper_func);
    }
    Ok(())
}

#[derive(Debug)]
pub struct BitwiseOr;

impl SimpleWord for BitwiseOr {
    fn name(&self) -> ClarityName {
        "bit-or".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        traverse_bitwise("bit-or", generator, builder, arg_types)
    }
}

#[derive(Debug)]
pub struct BitwiseAnd;

impl SimpleWord for BitwiseAnd {
    fn name(&self) -> ClarityName {
        "bit-and".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        traverse_bitwise("bit-and", generator, builder, arg_types)
    }
}

#[derive(Debug)]
pub struct BitwiseXor;

impl SimpleWord for BitwiseXor {
    fn name(&self) -> ClarityName {
        "bit-xor".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        traverse_bitwise("bit-xor", generator, builder, arg_types)
    }
}

#[derive(Debug)]
pub struct BitwiseLShift;

impl SimpleWord for BitwiseLShift {
    fn name(&self) -> ClarityName {
        "bit-shift-left".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let func = generator.func_by_name("stdlib.bit-shift-left");
        builder.call(func);
        Ok(())
    }
}

#[derive(Debug)]
pub struct BitwiseRShift;

impl SimpleWord for BitwiseRShift {
    fn name(&self) -> ClarityName {
        "bit-shift-right".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let type_suffix = match arg_types[0] {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                return Err(GeneratorError::TypeError(
                    "invalid type for shift".to_string(),
                ));
            }
        };

        let helper = generator.func_by_name(&format!("stdlib.bit-shift-right-{type_suffix}"));

        builder.call(helper);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Xor;

impl SimpleWord for Xor {
    fn name(&self) -> ClarityName {
        "xor".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // xor is a proxy call to bit-xor since they share the same implementation.
        traverse_bitwise("bit-xor", generator, builder, arg_types)
    }
}

#[cfg(test)]
mod test {
    use crate::tools::{crosscheck, evaluate};

    #[test]
    fn test_bitwise_and() {
        crosscheck(
            "
(define-public (assert)
  (ok (bit-and 3 3)))

(assert)",
            evaluate("(ok 3)"),
        )
    }

    #[test]
    fn test_bitwise_not() {
        crosscheck(
            "
(define-public (assert)
  (ok (bit-not 3)))

(assert)",
            evaluate("(ok -4)"),
        )
    }

    #[test]
    fn test_bitwise_or() {
        crosscheck(
            "
(define-public (assert)
  (ok (bit-or 1 2 3)))

(assert)",
            evaluate("(ok 3)"),
        )
    }

    #[test]
    fn test_bit_shift_left() {
        crosscheck(
            "
(define-public (assert)
  (ok (bit-shift-left 3 u1)))

(assert)",
            evaluate("(ok 6)"),
        )
    }

    #[test]
    fn test_bit_shift_right() {
        crosscheck(
            "
(define-public (assert)
  (ok (bit-shift-right 6 u1)))

(assert)",
            evaluate("(ok 3)"),
        )
    }

    #[test]
    fn test_bitwise_xor() {
        crosscheck(
            "
(define-public (assert)
  (ok (bit-xor 3 2)))

(assert)",
            evaluate("(ok 1)"),
        )
    }
}
