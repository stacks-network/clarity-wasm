use clar2wasm::tools::crosscheck;
use clarity::vm::types::{BuffData, SequenceData, TupleData, TupleTypeSignature, TypeSignature};
use clarity::vm::{ClarityName, Value};
use hex::FromHex as _;
use prop::sample::SizeRange;
use proptest::prelude::*;

use crate::{prop_signature, type_string, PropValue};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn deserialize_fixed_tuple_skip_one_after(val in PropValue::any()) {
        // A tuple { a: 42, b: <val>} where val should be added after the bytes
        let mut data = Vec::from_hex("0c000000020161000000000000000000000000000000002a0162").unwrap();

        val.0.serialize_write(&mut data).unwrap();
        let serialized = PropValue::from(Value::Sequence(SequenceData::Buffer(BuffData { data })));

        crosscheck(
            &format!("(from-consensus-buff? {{a: int}} {serialized})"),
            Ok(Some(
                Value::some(Value::Tuple(
                    TupleData::from_data(vec![("a".into(), Value::Int(42))]).unwrap(),
                ))
                .unwrap(),
            )),
        );
    }

    #[test]
    fn deserialize_fixed_tuple_skip_one_before(val in PropValue::any().no_shrink()) {
        // A tuple { a: <val>, b: 42 } where val should be added between the bytes
        let mut data = Vec::new();
        val.0.serialize_write(&mut data).unwrap();

        crosscheck(
            &format!("(from-consensus-buff? {{b: int}} 0x0c000000020161{}0162000000000000000000000000000000002a)", hex::encode(data)),
            Ok(Some(
                Value::some(Value::Tuple(
                    TupleData::from_data(vec![("b".into(), Value::Int(42))]).unwrap(),
                ))
                .unwrap(),
            )),
        );
    }

    #[test]
    fn deserialize_tuple_with_skippable_fields(
        (ty, tuple1) in tuple_and_signature("[a-z]{1,2}", 3),
        (_, tuple2) in tuple_and_signature("[a-z]{3,5}", 2..4)
    ) {
        let deserializable = tuple1.0.clone().expect_tuple().unwrap();
        let skippable = tuple2.0.expect_tuple().unwrap();

        let merged_tuple = match TupleData::shallow_merge(deserializable, skippable) {
            Ok(merged) => PropValue::from(Value::from(merged)),
            Err(_) => {
            // Skip for the rare cases where we generate a tuple too big
            return Ok(());
            }
        };

        let mut data = Vec::new();
        if merged_tuple.0.serialize_write(&mut data).is_err() {
            // Skip for the rare cases where we generate a tuple too big
            return Ok(());
        }

        crosscheck(
            &format!(
                "(from-consensus-buff? {} 0x{})",
                type_string(&ty),
                hex::encode(data)
            ),
            Ok(Some(Value::some(tuple1.into()).unwrap())),
        );
    }
}

fn tuple_and_signature(
    field_names: impl Strategy<Value = String>,
    elements: impl Into<SizeRange>,
) -> impl Strategy<Value = (TypeSignature, PropValue)> {
    prop::collection::vec(
        (
            field_names.prop_map(|n| ClarityName::try_from(n).unwrap()),
            prop_signature(),
        ),
        elements,
    )
    .prop_filter_map("Invalid TupleTypeSignature", |v| {
        TupleTypeSignature::try_from(v)
            .ok()
            .map(TypeSignature::from)
    })
    .prop_ind_flat_map2(PropValue::from_type)
}
