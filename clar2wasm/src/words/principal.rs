use clarity::{
    address::{
        C32_ADDRESS_VERSION_MAINNET_MULTISIG, C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
        C32_ADDRESS_VERSION_TESTNET_MULTISIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
    },
    vm::{ClarityName, SymbolicExpression},
};
use walrus::{
    ir::{BinaryOp, InstrSeqType, MemArg},
    ValType,
};

use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

use super::Word;

#[derive(Debug)]
pub struct IsStandard;

impl Word for IsStandard {
    fn name(&self) -> ClarityName {
        "is-standard".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // Traverse the principal
        generator.traverse_expr(builder, args.get_expr(0)?)?;

        // Drop the length
        builder.drop();

        // Read the version byte from the principal in memory
        builder.load(
            generator.get_memory(),
            walrus::ir::LoadKind::I32_8 {
                kind: walrus::ir::ExtendedLoad::ZeroExtend,
            },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Save the version byte in a local
        let version_local = generator.module.locals.add(ValType::I32);
        builder.local_tee(version_local);

        // TODO: It would be nice if this was a global variable that gets set
        //       at compile time, instead of requiring a host-interface call.
        // Check if we are in mainnet (leaves a boolean on the stack)
        builder.call(generator.func_by_name("is_in_mainnet"));

        builder.if_else(
            InstrSeqType::new(
                &mut generator.module.types,
                &[ValType::I32],
                &[ValType::I32],
            ),
            |then| {
                then.i32_const(C32_ADDRESS_VERSION_MAINNET_MULTISIG as i32)
                    .binop(BinaryOp::I32Eq);
                then.local_get(version_local)
                    .i32_const(C32_ADDRESS_VERSION_MAINNET_SINGLESIG as i32)
                    .binop(BinaryOp::I32Eq);
                then.binop(BinaryOp::I32Or);
            },
            |else_| {
                else_
                    .i32_const(C32_ADDRESS_VERSION_TESTNET_MULTISIG as i32)
                    .binop(BinaryOp::I32Eq);
                else_
                    .local_get(version_local)
                    .i32_const(C32_ADDRESS_VERSION_TESTNET_SINGLESIG as i32)
                    .binop(BinaryOp::I32Eq);
                else_.binop(BinaryOp::I32Or);
            },
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct Construct;

impl Word for Construct {
    fn name(&self) -> ClarityName {
        "principal-construct?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // Traverse the version byte
        generator.traverse_expr(builder, args.get_expr(0)?)?;
        // [ version_offset, version_length ]

        // Traverse the public key hash
        generator.traverse_expr(builder, args.get_expr(1)?)?;
        // [ version_offset, version_length, pkhash_offset, pkhash_length ]

        if let Some(contract) = args.get(2) {
            // Push a 1 to indicate that a contract name was passed
            builder.i32_const(1);

            // Traverse the contract name (if it exists)
            generator.traverse_expr(builder, contract)?;
        } else {
            // Else push a 0 to indicate that no contract name was passed, and
            // two placeholder 0s.
            builder.i32_const(0).i32_const(0).i32_const(0);
        }
        // [ version_offset, version_length,
        //   pkhash_offset, pkhash_length,
        //   contract_present, contract_offset, contract_length ]

        // Call the principal-construct function in the stdlib
        builder.call(generator.func_by_name("stdlib.principal-construct"));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::{
        types::{PrincipalData, TupleData},
        Value,
    };

    use crate::tools::evaluate;

    //- is-standard

    #[test]
    fn test_is_standard() {
        assert_eq!(
            evaluate("(is-standard 'STB44HYPYAT2BB2QE513NSP81HTMYWBJP02HPGK6)"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_is_standard_contract() {
        assert_eq!(
            evaluate("(is-standard 'STB44HYPYAT2BB2QE513NSP81HTMYWBJP02HPGK6.foo)"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_is_standard_multisig() {
        assert_eq!(
            evaluate("(is-standard 'SN3X6QWWETNBZWGBK6DRGTR1KX50S74D340JWTSC7)"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_is_standard_mainnet() {
        assert_eq!(
            evaluate("(is-standard 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY)"),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_is_standard_mainnet_contract() {
        assert_eq!(
            evaluate("(is-standard 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY.foo)"),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_is_standard_mainnet_multisig() {
        assert_eq!(
            evaluate("(is-standard 'SM3X6QWWETNBZWGBK6DRGTR1KX50S74D341M9C5X7)"),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_is_standard_other() {
        assert_eq!(
            evaluate("(is-standard 'SZ2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQ9H6DPR)"),
            Some(Value::Bool(false))
        );
    }

    //- principal-construct?

    #[test]
    fn test_construct_standard() {
        assert_eq!(
            evaluate("(principal-construct? 0x1a 0xfa6bf38ed557fe417333710d6033e9419391a320)"),
            Some(
                Value::okay(
                    PrincipalData::parse("ST3X6QWWETNBZWGBK6DRGTR1KX50S74D3425Q1TPK")
                        .unwrap()
                        .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_contract() {
        assert_eq!(
            evaluate(
                r#"(principal-construct? 0x1a 0xfa6bf38ed557fe417333710d6033e9419391a320 "foo")"#
            ),
            Some(
                Value::okay(
                    PrincipalData::parse("ST3X6QWWETNBZWGBK6DRGTR1KX50S74D3425Q1TPK.foo")
                        .unwrap()
                        .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_mainnet() {
        assert_eq!(
            evaluate("(principal-construct? 0x16 0xfa6bf38ed557fe417333710d6033e9419391a320)"),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(0)),
                        (
                            "value".into(),
                            Value::some(
                                PrincipalData::parse("SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY")
                                    .unwrap()
                                    .into()
                            )
                            .unwrap()
                        )
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_mainnet_contract() {
        assert_eq!(
            evaluate(
                r#"(principal-construct? 0x16 0xfa6bf38ed557fe417333710d6033e9419391a320 "foo")"#
            ),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(0)),
                        (
                            "value".into(),
                            Value::some(
                                PrincipalData::parse(
                                    "SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY.foo"
                                )
                                .unwrap()
                                .into()
                            )
                            .unwrap()
                        )
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_empty_version() {
        assert_eq!(
            evaluate("(principal-construct? 0x 0xfa6bf38ed557fe417333710d6033e9419391a320)"),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(1)),
                        ("value".into(), Value::none())
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_short_hash() {
        assert_eq!(
            evaluate("(principal-construct? 0x16 0xfa6bf38ed557fe417333710d6033e9419391a3)"),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(1)),
                        ("value".into(), Value::none())
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_high_version() {
        assert_eq!(
            evaluate("(principal-construct? 0x20 0xfa6bf38ed557fe417333710d6033e9419391a320)"),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(1)),
                        ("value".into(), Value::none())
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_empty_contract() {
        assert_eq!(
            evaluate(r#"(principal-construct? 0x1a 0xfa6bf38ed557fe417333710d6033e9419391a320 "")"#),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(2)),
                        ("value".into(), Value::none())
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn test_construct_illegal_contract() {
        assert_eq!(
            evaluate(r#"(principal-construct? 0x1a 0xfa6bf38ed557fe417333710d6033e9419391a320 "foo[")"#),
            Some(
                Value::error(
                    TupleData::from_data(vec![
                        ("error_code".into(), Value::UInt(2)),
                        ("value".into(), Value::none())
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }
}
