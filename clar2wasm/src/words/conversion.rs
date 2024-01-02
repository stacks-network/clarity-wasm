use clarity::vm::types::{SequenceSubtype, StringSubtype, TypeSignature};

use super::SimpleWord;
use crate::wasm_generator::GeneratorError;

#[derive(Debug)]
pub struct StringToInt;

impl SimpleWord for StringToInt {
    fn name(&self) -> clarity::vm::ClarityName {
        "string-to-int?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let func_prefix = match &arg_types[0] {
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

impl SimpleWord for StringToUint {
    fn name(&self) -> clarity::vm::ClarityName {
        "string-to-uint?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let func_prefix = match arg_types[0] {
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

impl SimpleWord for IntToAscii {
    fn name(&self) -> clarity::vm::ClarityName {
        "int-to-ascii".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let type_prefix = match arg_types[0] {
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

impl SimpleWord for IntToUtf8 {
    fn name(&self) -> clarity::vm::ClarityName {
        "int-to-utf8".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let type_prefix = match arg_types[0] {
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
