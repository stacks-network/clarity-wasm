use clar2wasm::tools::crosscheck_with_amount;
use clarity::vm::types::TupleData;
use clarity::vm::{ClarityName, Value};
use proptest::prelude::*;

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
}
