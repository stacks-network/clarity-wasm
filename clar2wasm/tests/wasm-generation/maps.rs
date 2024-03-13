use clar2wasm::tools::crosscheck;
use clarity::vm::types::{TupleData, TypeSignature};
use clarity::vm::{ClarityName, Value};
use proptest::prelude::*;

use crate::{prop_signature, type_string, PropValue};

fn map_type() -> impl Strategy<Value = (TypeSignature, TypeSignature)> {
    (prop_signature(), prop_signature())
}

fn map_entry(
    key: TypeSignature,
    value: TypeSignature,
) -> impl Strategy<Value = (PropValue, PropValue)> {
    (PropValue::from_type(key), PropValue::from_type(value))
}

fn map_entries(
    key: TypeSignature,
    value: TypeSignature,
    items: usize,
) -> impl Strategy<Value = Vec<(PropValue, PropValue)>> {
    prop::collection::vec(map_entry(key, value), items)
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_get_empty_map((kty, vty, k) in map_type().prop_flat_map(|(kty, vty)| (Just(kty.clone()), Just(vty), PropValue::from_type(kty)))) {
        let snippet = format!("(define-map test-map {} {}) (map-get? test-map {k})", type_string(&kty), type_string(&vty));
        crosscheck(&snippet, Ok(Some(Value::none())))
    }

    #[test]
    fn define_insert_get_map(
        (kty, vty, entries, random_key)
            in (map_type(), 1usize..=20)
                .prop_flat_map(|((kty, vty), size)| {
                    (Just(kty.clone()), Just(vty.clone()), map_entries(kty.clone(), vty, size), PropValue::from_type(kty))
    })) {
        // We generate here a snippet that looks like
        // ```
        // (define-map test-map {key type} {value type})
        // {
        //    a: (list (map-insert test-map {key0} {value0}) (map-insert test-map {key1} {value1}) ...)
        //    b: (list (map-get? test-map {key0}) (map-get? test-map {key1}) ... (map-get? test-map {random-key})))
        // }

        // will contain results of map-insert
        let mut expected_insert: Vec<Value> = Vec::with_capacity(entries.len());
        // will contain results of map-get?
        let mut expected_get: Vec<Value> = Vec::with_capacity(entries.len() + 1);
        // should be a hashset of the currently defined keys, but Value doesn't implement Hash
        let mut defined_keys = Vec::with_capacity(entries.len());

        let mut snippet_tuple_a = String::from("(list");
        let mut snippet_tuple_b = String::from("(list");

        for (i, (k, v)) in entries.iter().enumerate() {
            snippet_tuple_a.push_str(&format!(" (map-insert test-map {k} {v})"));
            snippet_tuple_b.push_str(&format!(" (map-get? test-map {k})"));
            expected_insert.push(Value::Bool(if defined_keys.contains(&k.0) {
                false
            } else {
                defined_keys.push(k.0.clone());
                true
            }));
            expected_get.push(
                entries[..=i]
                    .iter()
                    .find_map(|(ke, ve)| (k == ke).then(|| Value::some(ve.0.clone()).unwrap()))
                    .unwrap(),
            );
        }

        snippet_tuple_a.push(')');

        // get a random key for fun
        snippet_tuple_b.push_str(&format!(" (map-get? test-map {random_key}))"));
        expected_get.push(
            entries
                .into_iter()
                .find_map(|(key, val)| (random_key == key).then(|| Value::some(val.0).unwrap()))
                .unwrap_or_else(Value::none),
        );

        let snippet = format!(
            "(define-map test-map {} {}) {{a: {snippet_tuple_a}, b: {snippet_tuple_b}}}",
            type_string(&kty),
            type_string(&vty)
        );

        let expected = Value::from(
            TupleData::from_data(vec![
                (
                    ClarityName::from("a"),
                    Value::cons_list_unsanitized(expected_insert).unwrap(),
                ),
                (
                    ClarityName::from("b"),
                    Value::cons_list_unsanitized(expected_get).unwrap(),
                ),
            ])
            .unwrap(),
        );

        crosscheck(&snippet, Ok(Some(expected)));
    }
}
