use clarity::vm::types::{TypeSignature, MAX_VALUE_SIZE};
use walrus::ir::InstrSeqType;

use super::Word;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct To;

impl Word for To {
    fn name(&self) -> clarity::vm::ClarityName {
        "to-consensus-buff?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_args(builder, args)?;

        let ty = generator
            .get_expr_type(args.get_expr(0)?)
            .expect("to-consensus-buff? value expression must be typed")
            .clone();

        // Save the offset (current stack pointer) into a local.
        // This is where we will serialize the value to.
        let offset = generator.module.locals.add(walrus::ValType::I32);
        let length = generator.module.locals.add(walrus::ValType::I32);
        builder
            .global_get(generator.stack_pointer)
            .local_set(offset);

        // Write the serialized value to the top of the call stack
        generator.serialize_to_memory(builder, offset, 0, &ty)?;

        builder.local_set(length);

        // Check if the serialized value size < MAX_VALUE_SIZE
        builder
            .local_get(length)
            .i32_const(MAX_VALUE_SIZE as i32)
            .binop(walrus::ir::BinaryOp::I32LeU)
            .if_else(
                InstrSeqType::new(
                    &mut generator.module.types,
                    &[],
                    &[
                        walrus::ValType::I32,
                        walrus::ValType::I32,
                        walrus::ValType::I32,
                    ],
                ),
                |then| {
                    then.local_get(offset)
                        .local_get(length)
                        .binop(walrus::ir::BinaryOp::I32Add)
                        .global_set(generator.stack_pointer);

                    then.i32_const(1).local_get(offset).local_get(length);
                },
                |else_| {
                    else_.i32_const(0).i32_const(0).i32_const(0);
                },
            );

        Ok(())
    }
}

#[derive(Debug)]
pub struct From;

impl Word for From {
    fn name(&self) -> clarity::vm::ClarityName {
        "from-consensus-buff?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // Rather than parsing the type from args[0], we can just use the type
        // of this expression.
        let ty = generator
            .get_expr_type(_expr)
            .expect("from-consensus-buff? value expression must be typed")
            .clone();
        let value_ty = if let TypeSignature::OptionalType(inner) = ty {
            *inner
        } else {
            return Err(GeneratorError::TypeError(
                "from-consensus-buff? value expression must be an optional type".to_string(),
            ));
        };

        // Traverse the input buffer, leaving the offset and length on the stack.
        generator.traverse_expr(builder, args.get_expr(1)?)?;

        let offset = generator.module.locals.add(walrus::ValType::I32);
        let length = generator.module.locals.add(walrus::ValType::I32);
        builder.local_set(length);
        builder.local_set(offset);

        generator.deserialize_from_memory(builder, offset, length, 0, &value_ty)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::{BuffData, SequenceData};
    use clarity::vm::Value;
    use hex::FromHex as _;

    use crate::tools::evaluate;

    #[test]
    fn to_consensus_buff_int() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? 42)"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("000000000000000000000000000000002a").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_uint() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? u42)"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("010000000000000000000000000000002a").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_bool() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? true)"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("03").unwrap()
                })))
                .unwrap()
            )
        );
        assert_eq!(
            evaluate(r#"(to-consensus-buff? false)"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("04").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_optional() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? none)"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("09").unwrap()
                })))
                .unwrap()
            )
        );
        assert_eq!(
            evaluate(r#"(to-consensus-buff? (some 42))"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("0a000000000000000000000000000000002a").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_response() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? (ok 42))"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("07000000000000000000000000000000002a").unwrap()
                })))
                .unwrap()
            )
        );
        assert_eq!(
            evaluate(r#"(to-consensus-buff? (err u123))"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("08010000000000000000000000000000007b").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_tuple() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? {foo: 123, bar: u789})"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("0c0000000203626172010000000000000000000000000000031503666f6f000000000000000000000000000000007b").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_string_ascii() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? "Hello, World!")"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("0d0000000d48656c6c6f2c20576f726c6421").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_buffer() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? 0x12345678)"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("020000000412345678").unwrap()
                })))
                .unwrap()
            )
        );
    }

    #[test]
    fn to_consensus_buff_list() {
        assert_eq!(
            evaluate(r#"(to-consensus-buff? (list 1 2 3 4))"#),
            Some(
                Value::some(Value::Sequence(SequenceData::Buffer(BuffData {
                    data: Vec::from_hex("0b000000040000000000000000000000000000000001000000000000000000000000000000000200000000000000000000000000000000030000000000000000000000000000000004").unwrap()
                })))
                .unwrap()
            )
        );
    }

    //--- `from-consensus-buff?` tests

    #[test]
    fn from_consensus_buff_int() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? int 0x000000000000000000000000000001e240)"#),
            Some(Value::some(Value::Int(123456)).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_uint() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? uint 0x010000000000000000000000000001e240)"#),
            Some(Value::some(Value::UInt(123456)).unwrap())
        );
    }
}
