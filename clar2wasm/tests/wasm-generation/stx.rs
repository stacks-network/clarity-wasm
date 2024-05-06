use clar2wasm::tools::crosscheck_with_amount;
use clarity::vm::types::{TupleData, TypeSignature};
use clarity::vm::{ClarityName, Value};
use proptest::prelude::*;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn stx_balance_burn_balance(amount in any::<u128>()) {


        let snippet = format!(r#"
            {{
                a-balance1: (stx-get-balance 'S1G2081040G2081040G2081040G208105NK8PE5),
                b-burn: (stx-burn? u{amount} 'S1G2081040G2081040G2081040G208105NK8PE5),
                c-balance2: (stx-get-balance 'S1G2081040G2081040G2081040G208105NK8PE5),
            }}
        "#);

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("a-balance1"),
                    Value::UInt(amount),
                ),
                (
                    ClarityName::from("b-burn"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("c-balance2"),
                    Value::UInt(0),
                ),
            ])
            .unwrap(),
        );

        crosscheck_with_amount(&snippet, amount, Ok(Some(expected)));
    }

    #[test]
    fn stx_balance_transfer_balance(
        amount in any::<u128>(),
        new_owner in PropValue::from_type(TypeSignature::PrincipalType)

    ) {


        let snippet = format!(r#"
            {{
                a-balance-before: (stx-get-balance 'S1G2081040G2081040G2081040G208105NK8PE5),
                b-transfer: (stx-transfer? u{amount} 'S1G2081040G2081040G2081040G208105NK8PE5 {new_owner}),
                c-balance-former: (stx-get-balance 'S1G2081040G2081040G2081040G208105NK8PE5),
                d-balance-new: (stx-get-balance {new_owner}),
            }}
        "#);

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("a-balance-before"),
                    Value::UInt(amount),
                ),
                (
                    ClarityName::from("b-transfer"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("c-balance-former"),
                    Value::UInt(0),
                ),
                (
                    ClarityName::from("d-balance-new"),
                    Value::UInt(amount),
                ),
            ])
            .unwrap(),
        );

        crosscheck_with_amount(&snippet, amount, Ok(Some(expected)));
    }
}
