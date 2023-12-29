use clarity::vm::types::{SequenceSubtype, StringSubtype, TypeSignature};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError};

#[derive(Debug)]
pub struct StringToInt;

impl ComplexWord for StringToInt {
    fn name(&self) -> clarity::vm::ClarityName {
        "string-to-int?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_args(builder, args)?;

        let func_prefix = match generator
            .get_expr_type(args.get_expr(0)?)
            .expect("string-to-int? argument should have a type")
        {
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                "string"
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                "utf8"
            }
            _ => {
                return Err(GeneratorError::TypeError(
                    "impossible type for string-to-int?".to_owned(),
                ))
            }
        };

        let func = generator.func_by_name(&format!("stdlib.{func_prefix}-to-int"));
        builder.call(func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct StringToUint;

impl ComplexWord for StringToUint {
    fn name(&self) -> clarity::vm::ClarityName {
        "string-to-uint?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_args(builder, args)?;

        let func_prefix = match generator
            .get_expr_type(args.get_expr(0)?)
            .expect("string-to-int? argument should have a type")
        {
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                "string"
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                "utf8"
            }
            _ => {
                return Err(GeneratorError::TypeError(
                    "impossible type for string-to-int?".to_owned(),
                ))
            }
        };

        let func = generator.func_by_name(&format!("stdlib.{func_prefix}-to-uint"));

        builder.call(func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct IntToAscii;

impl ComplexWord for IntToAscii {
    fn name(&self) -> clarity::vm::ClarityName {
        "int-to-ascii".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_args(builder, args)?;

        let input = args.get_expr(0)?;
        let ty = generator
            .get_expr_type(input)
            .expect("int-to-ascii input must be typed");
        let type_prefix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                return Err(GeneratorError::InternalError(
                    "invalid type for int-to-ascii".to_owned(),
                ));
            }
        };

        let func = generator.func_by_name(&format!("stdlib.{type_prefix}-to-string"));

        builder.call(func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct IntToUtf8;

impl ComplexWord for IntToUtf8 {
    fn name(&self) -> clarity::vm::ClarityName {
        "int-to-utf8".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        generator.traverse_args(builder, args)?;

        let input = args.get_expr(0)?;
        let ty = generator
            .get_expr_type(input)
            .expect("int-to-utf8 input must be typed");
        let type_prefix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                return Err(GeneratorError::InternalError(
                    "invalid type for int-to-utf8".to_owned(),
                ));
            }
        };

        let func = generator.func_by_name(&format!("stdlib.{type_prefix}-to-utf8"));

        builder.call(func);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::{ASCIIData, CharType, SequenceData, UTF8Data};
    use clarity::vm::Value;

    use crate::tools::evaluate;

    #[test]
    fn valid_string_to_int() {
        assert_eq!(
            evaluate(r#"(string-to-int? "1234567")"#),
            Some(Value::some(Value::Int(1234567)).unwrap())
        )
    }

    #[test]
    fn valid_negative_string_to_int() {
        assert_eq!(
            evaluate(r#"(string-to-int? "-1234567")"#),
            Some(Value::some(Value::Int(-1234567)).unwrap())
        )
    }

    #[test]
    fn invalid_string_to_int() {
        assert_eq!(
            evaluate(r#"(string-to-int? "0xabcd")"#),
            Some(Value::none())
        )
    }

    #[test]
    fn valid_string_to_uint() {
        assert_eq!(
            evaluate(r#"(string-to-uint? "98765")"#),
            Some(Value::some(Value::UInt(98765)).unwrap())
        )
    }

    #[test]
    fn invalid_string_to_uint() {
        assert_eq!(
            evaluate(r#"(string-to-uint? "0xabcd")"#),
            Some(Value::none())
        )
    }

    #[test]
    fn valid_utf8_to_int() {
        assert_eq!(
            evaluate(r#"(string-to-int? u"1234567")"#),
            Some(Value::some(Value::Int(1234567)).unwrap())
        )
    }

    #[test]
    fn valid_negative_utf8_to_int() {
        assert_eq!(
            evaluate(r#"(string-to-int? u"-1234567")"#),
            Some(Value::some(Value::Int(-1234567)).unwrap())
        )
    }

    #[test]
    fn invalid_utf8_to_int() {
        assert_eq!(
            evaluate(r#"(string-to-int? u"0xabcd")"#),
            Some(Value::none())
        )
    }

    #[test]
    fn valid_utf8_to_uint() {
        assert_eq!(
            evaluate(r#"(string-to-uint? u"98765")"#),
            Some(Value::some(Value::UInt(98765)).unwrap())
        )
    }

    #[test]
    fn invalid_utf8_to_uint() {
        assert_eq!(
            evaluate(r#"(string-to-uint? u"0xabcd")"#),
            Some(Value::none())
        )
    }

    #[test]
    fn uint_to_string() {
        assert_eq!(
            evaluate(r#"(int-to-ascii u42)"#),
            Some(Value::Sequence(SequenceData::String(CharType::ASCII(
                ASCIIData {
                    data: "42".bytes().collect()
                }
            ))))
        )
    }

    #[test]
    fn positive_int_to_string() {
        assert_eq!(
            evaluate(r#"(int-to-ascii 2048)"#),
            Some(Value::Sequence(SequenceData::String(CharType::ASCII(
                ASCIIData {
                    data: "2048".bytes().collect()
                }
            ))))
        )
    }

    #[test]
    fn negative_int_to_string() {
        assert_eq!(
            evaluate(r#"(int-to-ascii -2048)"#),
            Some(Value::Sequence(SequenceData::String(CharType::ASCII(
                ASCIIData {
                    data: "-2048".bytes().collect()
                }
            ))))
        )
    }

    #[test]
    fn uint_to_utf8() {
        assert_eq!(
            evaluate(r#"(int-to-utf8 u42)"#),
            Some(Value::Sequence(SequenceData::String(CharType::UTF8(
                UTF8Data {
                    data: "42".bytes().map(|b| vec![b]).collect()
                }
            ))))
        )
    }

    #[test]
    fn positive_int_to_utf8() {
        assert_eq!(
            evaluate(r#"(int-to-utf8 2048)"#),
            Some(Value::Sequence(SequenceData::String(CharType::UTF8(
                UTF8Data {
                    data: "2048".bytes().map(|b| vec![b]).collect()
                }
            ))))
        )
    }

    #[test]
    fn negative_int_to_utf8() {
        assert_eq!(
            evaluate(r#"(int-to-utf8 -2048)"#),
            Some(Value::Sequence(SequenceData::String(CharType::UTF8(
                UTF8Data {
                    data: "-2048".bytes().map(|b| vec![b]).collect()
                }
            ))))
        )
    }
}
