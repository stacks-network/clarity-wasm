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
    use clarity::vm::types::{BuffData, PrincipalData, SequenceData, TupleData};
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

    #[test]
    fn from_consensus_buff_optional_int_none() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (optional int) 0x09)"#),
            Some(Value::some(Value::none()).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_optional_bad_prefix() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (optional int) 0x00ffffffffffffffffffffffffffffffd6)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_optional_int_some() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (optional int) 0x0a00ffffffffffffffffffffffffffffffd6)"#
            ),
            Some(Value::some(Value::some(Value::Int(-42)).unwrap()).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_optional_bool_some() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (optional bool) 0x0a03)"#),
            Some(Value::some(Value::some(Value::Bool(true)).unwrap()).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_optional_int_some_invalid() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (optional int) 0x0a02ffffffffffffffffffffffffffffffd6)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_optional_int_some_long() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (optional int) 0x0a00ffffffffffffffffffffffffffffffd600)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_optional_int_some_short() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (optional int) 0x0a00ffffffffffffffffffffffffffffd6)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_response_simple_ok() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (response int int) 0x07000000000000000000000000000000007b)"#
            ),
            Some(Value::some(Value::okay(Value::Int(123)).unwrap()).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_response_simple_err() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (response int uint) 0x0801000000000000000000000000000001c8)"#
            ),
            Some(Value::some(Value::err_uint(456)).unwrap())
        );
    }

    #[test]
    fn from_consensus_buff_response_bad_prefix() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (response int int) 0x000000000000000000000000000000007b)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_response_short() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (response int int) 0x070000000000000000000000000000007b)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_response_long() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (response int bool) 0x07000000000000000000000000000000007b03)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_response_ok() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (response (string-ascii 128) uint) 0x070d000000455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73)"#
            ),
            Some(
                Value::some(
                    Value::okay(
                        Value::string_ascii_from_bytes(
                            "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks"
                                .to_string()
                                .into_bytes()
                        )
                        .unwrap()
                    )
                    .unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_buffer_exact_size() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (buff 16) 0x0200000010000102030405060708090a0b0c0d0e0f)"#
            ),
            Some(
                Value::some(
                    Value::buff_from(vec![
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                        0x0c, 0x0d, 0x0e, 0x0f
                    ])
                    .unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_buffer_smaller_than_type() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (buff 16) 0x02000000080001020304050607)"#),
            Some(
                Value::some(
                    Value::buff_from(vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]).unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_buffer_smaller_than_size() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (buff 16) 0x020000000800010203040506)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_buffer_larger_than_size() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (buff 16) 0x0200000008000102030405060708)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_buffer_larger_than_type() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (buff 8) 0x0200000009000102030405060708)"#),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_string_ascii_exact_size() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (string-ascii 13) 0x0d0000000d48656c6c6f2c20776f726c6421)"#
            ),
            Some(
                Value::some(
                    Value::string_ascii_from_bytes("Hello, world!".to_string().into_bytes())
                        .unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_string_ascii_smaller_than_type() {
        assert_eq!(
            evaluate(r#"(from-consensus-buff? (string-ascii 13) 0x0d00000008686920776f726c64)"#),
            Some(
                Value::some(
                    Value::string_ascii_from_bytes("hi world".to_string().into_bytes()).unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_string_ascii_smaller_than_size() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (string-ascii 13) 0x0d0000000d48656c6c6f2c20776f726c64)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_string_ascii_larger_than_size() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (string-ascii 13) 0x0d0000000d48656c6c6f2c20776f726c642121)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_string_ascii_larger_than_type() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (string-ascii 8) 0x0d0000000d48656c6c6f2c20776f726c6421)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_list_int() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (list 8 int) 0x0b00000003000000000000000000000000000000000100000000000000000000000000000000020000000000000000000000000000000003)"#
            ),
            Some(
                Value::some(
                    Value::cons_list_unsanitized(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
                        .unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_list_int_shorter_than_size() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (list 8 int) 0x0b0000000300000000000000000000000000000000010000000000000000000000000000000002)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_list_int_larger_than_size() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (list 8 int) 0x0b000000030000000000000000000000000000000001000000000000000000000000000000000200000000000000000000000000000000030000000000000000000000000000000004)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_list_int_larger_than_type() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (list 2 int) 0x0b000000040000000000000000000000000000000001000000000000000000000000000000000200000000000000000000000000000000030000000000000000000000000000000004)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_list_string() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? (list 8 (string-ascii 16)) 0x0b000000020d000000075361746f7368690d000000084e616b616d6f746f)"#
            ),
            Some(
                Value::some(
                    Value::cons_list_unsanitized(vec![
                        Value::string_ascii_from_bytes("Satoshi".to_string().into_bytes()).unwrap(),
                        Value::string_ascii_from_bytes("Nakamoto".to_string().into_bytes())
                            .unwrap()
                    ])
                    .unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_tuple_simple() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? {n: int} 0x0c00000001016e000000000000000000000000000000002a)"#
            ),
            Some(
                Value::some(Value::Tuple(
                    TupleData::from_data(vec![("n".into(), Value::Int(42))]).unwrap()
                ))
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_tuple_multiple() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? {my-number: int, a-string: (string-ascii 16), an-optional: (optional uint)} 0x0c0000000308612d737472696e670d0000000a7975702c2069742069730b616e2d6f7074696f6e616c09096d792d6e756d62657200ffffffffffffffffffffffffffffff85)"#
            ),
            Some(
                Value::some(Value::Tuple(
                    // {my-number: -123, a-string: "yup, it is", an-optional: none}
                    TupleData::from_data(vec![
                        ("my-number".into(), Value::Int(-123)),
                        (
                            "a-string".into(),
                            Value::string_ascii_from_bytes("yup, it is".to_string().into_bytes())
                                .unwrap()
                        ),
                        ("an-optional".into(), Value::none())
                    ])
                    .unwrap()
                ))
                .unwrap()
            )
        );
    }

    #[test]
    fn from_consensus_buff_tuple_extra_pair() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? {n: int} 0x0c000000020565787472610100000000000000000000000000000020016e000000000000000000000000000000002a)"#
            ),
            Some(Value::none())
        );
    }

    #[test]
    fn from_consensus_buff_tuple_missing_pair() {
        assert_eq!(
            evaluate(
                r#"(from-consensus-buff? {my-number: int, a-string: (string-ascii 16), an-optional: (optional uint)} 0x0c000000020b616e2d6f7074696f6e616c09096d792d6e756d62657200ffffffffffffffffffffffffffffff85)"#
            ),
            Some(Value::none())
        );
    }
}
