use clar2wasm::tools::crosscheck;
use proptest::prelude::*;

use clarity::vm::{
    types::{signatures::TypeSignature::PrincipalType, TupleData},
    ClarityName, Value,
};

use crate::{PropValue, TypePrinter};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn nft_mint_get_owner(
        nft in PropValue::any(),
        owner in PropValue::from_type(PrincipalType)
    ) {
        let snippet = format!(r#"
            (define-non-fungible-token stackaroo {})
            {{
                mint: (nft-mint? stackaroo {nft} {owner}),
                owner: (nft-get-owner? stackaroo {nft})
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
}
