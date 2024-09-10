//
// Proptests that should only be executed
// when running Clarity::V2 or Clarity::v3.
//
#[cfg(not(feature = "test-clarity-v1"))]
mod clarity_v2_v3 {
    use clar2wasm::tools::crosscheck;
    use clarity::vm::types::{
        BuffData, OptionalData, PrincipalData, QualifiedContractIdentifier, SequenceData,
        StandardPrincipalData, TupleData,
    };
    use clarity::vm::Value;
    use proptest::prelude::{Just, Strategy};
    use proptest::{prop_oneof, proptest};

    use crate::{buffer, runtime_config, PropValue};

    fn create_principal(version: u8, principal: &Vec<u8>, contract_name: Option<&str>) -> Value {
        if let Some(contract_name) = contract_name {
            // contract principal requested
            Value::Principal(PrincipalData::Contract(QualifiedContractIdentifier::new(
                StandardPrincipalData(
                    version,
                    principal
                        .as_slice()
                        .try_into()
                        .expect("slice bigger than 20 bytes"),
                ),
                contract_name.into(),
            )))
        } else {
            // standard principal requested
            Value::Principal(PrincipalData::Standard(StandardPrincipalData(
                version,
                principal
                    .as_slice()
                    .try_into()
                    .expect("slice bigger than 20 bytes"),
            )))
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

    fn create_error_destruct(hash_bytes: Value, version_byte: u8) -> Value {
        Value::error(
            TupleData::from_data(vec![
                ("hash-bytes".into(), hash_bytes),
                ("name".into(), Value::Optional(OptionalData { data: None })),
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

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_principal_construct(
            version_byte in 0x00..=0x1f,
            hash_bytes in buffer(20)
        ) {
            let expected_principal = create_principal(
                version_byte as u8,
                &hash_bytes.clone().expect_buff(20).unwrap(),
                None
            );

            let expected = match version_byte {
                // Valid range for version_bytes
                0x00..=0x1f => {
                    match version_byte {
                        // Since tests runs on a Testnet version,
                        // version_bytes single_sig (0x1A) || multi_sig (0x15), for Testnet,
                        // will return an Ok value.
                        0x1A | 0x15 => Value::okay(expected_principal),
                        _ => Ok(create_error_construct(0, Some(expected_principal))),
                    }
                },
                _ => Ok(create_error_construct(1, None)),
            }.unwrap();

            crosscheck(
                &format!("(principal-construct? 0x{:02X} {hash_bytes})", version_byte),
                Ok(Some(expected)),
            );
        }
    }

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_principal_destruct(
            version_byte in 0x00..=0x1f,
            hash_bytes in buffer(20)
        ) {
            let expected_principal = create_principal(
                version_byte as u8,
                &hash_bytes.clone().expect_buff(20).unwrap(),
                None
            );

            let expected = match version_byte {
                // Valid range for version_bytes
                0x00..=0x1f => {
                    match version_byte {
                        // Since tests runs on a Testnet version,
                        // version_bytes single_sig (0x1A) || multi_sig (0x15), for Testnet,
                        // will return an Ok value.
                        0x1A | 0x15 => Value::okay(
                            TupleData::from_data(vec![
                                ("hash-bytes".into(), hash_bytes),
                                ("name".into(), Value::Optional(OptionalData { data: None })),
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
                        _ => Ok(create_error_destruct(hash_bytes, version_byte as u8)),
                    }
                },
                _ => Ok(create_error_destruct(hash_bytes, version_byte as u8)),
            }.unwrap();

            crosscheck(
                &format!("(principal-destruct? {})", PropValue::from(expected_principal.clone())),
                Ok(Some(expected)),
            );
        }
    }

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crosscheck_is_standard(
            version_byte in 0x00..=0x1f,
            hash_bytes in buffer(20),
            contract in "([a-zA-Z](([a-zA-Z0-9]|[-])){0, 30})".prop_flat_map(|name| {
                prop_oneof![Just(Some(name)), Just(None)]
            })
        ) {
            let expected_principal = create_principal(
                version_byte as u8,
                &hash_bytes.expect_buff(20).unwrap(),
                contract.as_deref()
            );

            let principal_str = PropValue::from(expected_principal.clone()).to_string();
            let expected = matches!(principal_str.get(0..3), Some("'ST") | Some("'SN"));

            crosscheck(
                &format!("(is-standard {})", PropValue::from(expected_principal.clone())),
                Ok(Some(Value::Bool(expected))),
            );
        }
    }
}
