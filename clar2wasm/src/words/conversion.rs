use super::Word;

#[derive(Debug)]
pub struct StringToInt;

impl Word for StringToInt {
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

        let func = generator.func_by_name("stdlib.string-to-int");
        builder.call(func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct StringToUint;

impl Word for StringToUint {
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

        let func = generator.func_by_name("stdlib.string-to-uint");
        builder.call(func);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
}
