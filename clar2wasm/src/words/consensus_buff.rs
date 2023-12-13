use clarity::vm::types::{TypeSignature, MAX_VALUE_SIZE};
use walrus::ir::{BinaryOp, InstrSeqType};

use super::Word;
use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError,
    WasmGenerator,
};

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
        let wasm_result_ty = clar2wasm_ty(&ty);
        let value_ty = if let TypeSignature::OptionalType(ref inner) = ty {
            *inner.clone()
        } else {
            return Err(GeneratorError::TypeError(
                "from-consensus-buff? value expression must be an optional type".to_string(),
            ));
        };

        // Traverse the input buffer, leaving the offset and length on the stack.
        generator.traverse_expr(builder, args.get_expr(1)?)?;

        let offset = generator.module.locals.add(walrus::ValType::I32);
        let end = generator.module.locals.add(walrus::ValType::I32);
        builder
            .local_set(end)
            .local_tee(offset)
            .local_get(end)
            .binop(BinaryOp::I32Add)
            .local_set(end);

        generator.deserialize_from_memory(builder, offset, end, &value_ty)?;

        // If the entire buffer was not consumed, return none.
        builder
            .local_get(end)
            .local_get(offset)
            .binop(BinaryOp::I32Eq)
            .if_else(
                InstrSeqType::new(
                    &mut generator.module.types,
                    &wasm_result_ty,
                    &wasm_result_ty,
                ),
                |_| {
                    // Do nothing, leave the result as-is
                },
                |else_| {
                    // Drop the result and return none
                    drop_value(else_, &ty);
                    add_placeholder_for_clarity_type(else_, &ty);
                },
            );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::{BuffData, PrincipalData, SequenceData};
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
    fn from_consensus_buff_int_bad_prefix() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? int 0x0100000000000000000000000001e240)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_int_short() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? int 0x0000000000000000000000000001e240)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_int_long() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? int 0x000000000000000000000000000001e24000)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_uint() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? uint 0x010000000000000000000000000001e240)"#),
            Some(Value::some(Value::UInt(123456)).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_uint_bad_prefix() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? uint 0x0000000000000000000000000001e240)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_uint_short() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? uint 0x0100000000000000000000000001e240)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_uint_long() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? uint 0x010000000000000000000000000001e24000)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_standard_principal() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x051a7321b74e2b6a7e949e6c4ad313035b1665095017)"#
            ),
            Some(
                Value::some(Value::Principal(
                    PrincipalData::parse_standard_principal(
                        "ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5"
                    )
                    .unwrap()
                    .into()
                ))
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_principal_bad_prefix() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x071a7321b74e2b6a7e949e6c4ad313035b1665095017)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_standard_principal_short() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x051a7321b74e2b6a7e949e6c4ad313035b16650950)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_standard_principal_long() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x051a7321b74e2b6a7e949e6c4ad313035b1665095017ff)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_contract_principal() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x061a99e2ec69ac5b6e67b4e26edd0e2c1c1a6b9bbd230d66756e6374696f6e2d6e616d65)"#
            ),
            Some(
                Value::some(Value::Principal(
                    PrincipalData::parse_qualified_contract_principal(
                        "ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG.function-name"
                    )
                    .unwrap()
                ))
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_contract_principal_short() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x061a99e2ec69ac5b6e67b4e26edd0e2c1c1a6b9bbd230d66756e6374696f6e2d6e616d)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_contract_principal_long() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? principal 0x061a99e2ec69ac5b6e67b4e26edd0e2c1c1a6b9bbd230d66756e6374696f6e2d6e616d6565)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_bool_true() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? bool 0x03)"#),
            Some(Value::some(Value::Bool(true)).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_bool_false() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? bool 0x04)"#),
            Some(Value::some(Value::Bool(false)).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_bool_bad_prefix() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? bool 0x02)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_bool_short() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? bool 0x)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_bool_long() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? bool 0x0404)"#),
            Some(Value::none())
        );
    }
}
