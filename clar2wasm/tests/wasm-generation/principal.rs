//
// Proptests that should only be executed
// when running Clarity::V2 or Clarity::v3.
//
#[cfg(not(feature = "test-clarity-v1"))]
mod clarity_v2_v3 {
    use clar2wasm::tools::{crosscheck_with_network, Network};
    use clarity::address::AddressHashMode;
    use clarity::types::chainstate::{StacksAddress, StacksPrivateKey, StacksPublicKey};
    use clarity::util::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
    use clarity::vm::types::{
        ASCIIData, BuffData, CharType, OptionalData, PrincipalData, QualifiedContractIdentifier,
        SequenceData, StandardPrincipalData, TupleData,
    };
    use clarity::vm::Value;
    use clarity::{C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG};
    use proptest::prelude::{any, Just, Strategy};
    use proptest::string::string_regex;
    use proptest::{option, prop_oneof, proptest};

    use crate::{buffer, qualified_principal, runtime_config, standard_principal, PropValue};

    fn strategies_for_version_byte() -> impl Strategy<Value = i32> {
        prop_oneof![
            13 => Just(0x1A),
            13 => Just(0x14),
            12 => Just(0x15),
            12 => Just(0x16),
            50 => 0x00..=0x1F,
        ]
    }

    fn create_principal(version: u8, principal: &[u8], contract_name: Option<&str>) -> Value {
        let principal_data: [u8; 20] = principal
            .try_into()
            .expect("slice must be exactly 20 bytes");

        match contract_name {
            Some(contract_name) => {
                Value::Principal(PrincipalData::Contract(QualifiedContractIdentifier::new(
                    StandardPrincipalData(version, principal_data),
                    contract_name.into(),
                )))
            }
            None => Value::Principal(PrincipalData::Standard(StandardPrincipalData(
                version,
                principal_data,
            ))),
        }
    }

    fn create_error_construct(error_code: u8, principal_data: Option<Value>) -> Value {
        Value::error(
            TupleData::from_data(vec![
                ("error_code".into(), Value::UInt(error_code.into())),
                (
                    "value".into(),
                    Value::Optional(OptionalData {
                        data: principal_data.map(Box::new),
                    }),
                ),
            ])
            .unwrap()
            .into(),
        )
        .unwrap()
    }

    fn create_error_destruct(
        hash_bytes: Value,
        version_byte: u8,
        data: Option<Box<Value>>,
    ) -> Value {
        Value::error(
            TupleData::from_data(vec![
                ("hash-bytes".into(), hash_bytes),
                ("name".into(), Value::Optional(OptionalData { data })),
                (
                    "version".into(),
                    Value::Sequence(SequenceData::Buffer(BuffData {
                        data: vec![version_byte],
                    })),
                ),
            ])
            .unwrap()
            .into(),
        )
        .unwrap()
    }

    fn key_to_stacks_addr(address_version: u8, key: &StacksPrivateKey) -> StacksAddress {
        StacksAddress::from_public_keys(
            address_version,
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![StacksPublicKey::from_private(key)],
        )
        .unwrap()
    }

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_principal_construct(
            version_byte in strategies_for_version_byte(),
            hash_bytes in buffer(20),
            contract in option::of(string_regex("([a-zA-Z](([a-zA-Z0-9]|[-])){0,30})").unwrap())
        ) {
            let expected_principal = create_principal(
                version_byte as u8,
                &hash_bytes.clone().expect_buff(20).unwrap(),
                contract.as_deref(),
            );

            let expected = match version_byte {
                // version_bytes for Mainnet: single_sig (0x14) and multi_sig (0x16).
                // version_bytes for Testnet: single_sig (0x1A) and multi_sig (0x15).
                0x1A | 0x14 | 0x15 | 0x16 => Value::okay(expected_principal),
                // Special cases for invalid or out-of-range version_bytes
                0x00..=0x1F => Ok(create_error_construct(0, Some(expected_principal))),
                _ => panic!("Test case not handled for principal-construct?"),
            }.unwrap();

            let snippet = if let Some(contract) = &contract {
                format!("(principal-construct? 0x{:02X} {hash_bytes} \"{}\")", version_byte, contract)
            } else {
                format!("(principal-construct? 0x{:02X} {hash_bytes})", version_byte)
            };

            let crosscheck_for = |network: Network, expected: Value| {
                crosscheck_with_network(
                    network,
                    &snippet,
                    Ok(Some(expected)),
                );
            };

            match version_byte {
                0x14 | 0x16 => crosscheck_for(Network::Mainnet, expected.clone()),
                _ => crosscheck_for(Network::Testnet, expected.clone()),
            }
        }
    }

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_principal_destruct(
            version_byte in strategies_for_version_byte(),
            hash_bytes in buffer(20),
            contract in option::of(string_regex("([a-zA-Z](([a-zA-Z0-9]|[-])){0,30})").unwrap())
        ) {
            let expected_principal = create_principal(
                version_byte as u8,
                &hash_bytes.clone().expect_buff(20).unwrap(),
                contract.as_deref()
            );

            let data = contract.map(|ctc| Box::new(
                Value::Sequence(SequenceData::String(CharType::ASCII(ASCIIData {
                    data: ctc.into_bytes()
                })))
            ));

            let expected = match version_byte {
                // version_bytes for Mainnet: single_sig (0x14) and multi_sig (0x16).
                // version_bytes for Testnet: single_sig (0x1A) and multi_sig (0x15).
                0x1A | 0x14 | 0x15 | 0x16 => Value::okay(
                    TupleData::from_data(vec![
                        ("hash-bytes".into(), hash_bytes),
                        ("name".into(), Value::Optional(OptionalData { data })),
                        (
                            "version".into(),
                            Value::Sequence(SequenceData::Buffer(BuffData {
                                data: vec![version_byte as u8],
                            })),
                        ),
                    ])
                    .unwrap()
                    .into()
                ),
                // Special cases for invalid or out-of-range version_bytes
                0x00..=0x1F => Ok(create_error_destruct(hash_bytes, version_byte as u8, data)),
                _ => panic!("Test case not handled for principal-destruct?"),
            }.unwrap();

            let crosscheck_for = |network: Network, expected_principal: Value| {
                crosscheck_with_network(
                    network,
                    &format!("(principal-destruct? {})", PropValue::from(expected_principal)),
                    Ok(Some(expected)),
                );
            };

            match version_byte {
                0x14 | 0x16 => crosscheck_for(Network::Mainnet, expected_principal.clone()),
                _ => crosscheck_for(Network::Testnet, expected_principal.clone()),
            }
        }
    }

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_is_standard(
            principal in prop_oneof![standard_principal(), qualified_principal()].prop_map(PropValue)
        ) {
            let principal_str = principal.to_string();
            let expected_in_testnet = matches!(principal_str.get(0..3), Some("'ST") | Some("'SN"));
            let expected_in_mainnet = matches!(principal_str.get(0..3), Some("'SP") | Some("'SM"));

            let crosscheck_for = |network: Network, expected: bool| {
                crosscheck_with_network(
                    network,
                    &format!("(is-standard {})", principal),
                    Ok(Some(Value::Bool(expected))),
                );
            };

            crosscheck_for(Network::Mainnet, expected_in_mainnet);
            crosscheck_for(Network::Testnet, expected_in_testnet);
        }
    }

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_principal_of(
            (private_key, public_key) in proptest::collection::vec(any::<u8>(), 20).prop_map(|seed| {
                let private_key = Secp256k1PrivateKey::from_seed(&seed);
                let public_key = Secp256k1PublicKey::from_private(&private_key);
                (private_key, public_key)
            })
        ) {
            let snippet = format!("(principal-of? 0x{})", public_key.to_hex());

            let crosscheck_for = |network: Network, snippet: &str, private_key: &Secp256k1PrivateKey, address_version: u8| {
                let expected_principal = StandardPrincipalData::from(key_to_stacks_addr(address_version, private_key));
                crosscheck_with_network(
                    network,
                    snippet,
                    Ok(Some(Value::okay(expected_principal.into()).expect("Valid principal expected"))),
                );
            };

            crosscheck_for(
                Network::Testnet,
                &snippet,
                &private_key,
                C32_ADDRESS_VERSION_TESTNET_SINGLESIG,

            );

            crosscheck_for(
                Network::Mainnet,
                &snippet,
                &private_key,
                C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            );
        }
    }
}
