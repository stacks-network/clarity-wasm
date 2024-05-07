use clar2wasm::tools::crosscheck;
use clarity::vm::types::signatures::TypeSignature::PrincipalType;
use clarity::vm::types::TupleData;
use clarity::vm::{ClarityName, Value};
use proptest::prelude::*;

use crate::{PropValue, TypePrinter};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn nft_mint_get_owner(
        nft in PropValue::any(),
        owner in PropValue::from_type(PrincipalType),
    ) {
        let snippet = format!(r#"
            (define-non-fungible-token stackaroo {})
            {{
                mint: (nft-mint? stackaroo {nft} {owner}),
                owner: (nft-get-owner? stackaroo {nft}),
            }}
        "#, nft.type_string());

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("mint"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("owner"),
                    Value::some(owner.into()).unwrap(),
                ),
            ])
            .unwrap(),
        );

        crosscheck(&snippet, Ok(Some(expected)));
    }

    #[test]
    fn nft_mint_transfer_owner(
        nft in PropValue::any(),
        owner1 in PropValue::from_type(PrincipalType),
        owner2 in PropValue::from_type(PrincipalType),
    ) {
        let snippet = format!(r#"
            (define-non-fungible-token stackaroo {})
            {{
                a-mint: (nft-mint? stackaroo {nft} {owner1}),
                b-transfer: (nft-transfer? stackaroo {nft} {owner1} {owner2}),
                c-owner: (nft-get-owner? stackaroo {nft}),
            }}
        "#, nft.type_string());

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("a-mint"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("b-transfer"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("c-owner"),
                    Value::some(owner2.into()).unwrap(),
                ),
            ])
            .unwrap(),
        );

        crosscheck(&snippet, Ok(Some(expected)));
    }

    #[test]
    fn nft_mint_burn_get_owner(
        nft in PropValue::any(),
        owner in PropValue::from_type(PrincipalType),
    ) {
        let snippet = format!(r#"
            (define-non-fungible-token stackaroo {})
            {{
                a-mint: (nft-mint? stackaroo {nft} {owner}),
                b-burn: (nft-burn? stackaroo {nft} {owner}),
                c-owner: (nft-get-owner? stackaroo {nft}),
            }}
        "#, nft.type_string());

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("a-mint"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("b-burn"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("c-owner"),
                    Value::none(),
                ),
            ])
            .unwrap(),
        );

        crosscheck(&snippet, Ok(Some(expected)));
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn ft_mint_balance(
        total_supply in any::<u128>(),
        recipient in PropValue::from_type(PrincipalType),
    ) {
        let snippet = format!(r#"
            (define-fungible-token stackaroo u{total_supply})
            {{
                a-mint: (ft-mint? stackaroo u{total_supply} {recipient}),
                b-balance: (ft-get-balance stackaroo {recipient}),
            }}
        "#);

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("a-mint"),
                    Value::okay_true(),
                ),
                (
                    ClarityName::from("b-balance"),
                    Value::UInt(total_supply),
                ),
            ])
            .unwrap(),
        );

        crosscheck(&snippet, Ok(Some(expected)));
    }
}
