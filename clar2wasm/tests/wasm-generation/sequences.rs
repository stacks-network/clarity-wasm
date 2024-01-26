use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::prelude::*;

use crate::{prop_signature, PropValue};

proptest! {
    #[test]
    fn append_value_to_list(mut values in (prop_signature(), 1usize..32).prop_flat_map(|(ty, size)| PropValue::many_from_type(ty, size))) {
        let expected = Value::cons_list_unsanitized(values.iter().cloned().map(Value::from).collect()).unwrap();

        let elem = values.pop().unwrap();
        let values = PropValue::try_from(values).unwrap();

        crosscheck(&format!("(append {values} {elem})"), Ok(Some(expected)))
    }
}

proptest! {
    #[test]
    fn as_max_len_equal_max_len_is_some((max_len, value) in (0usize..=16).prop_ind_flat_map2(PropValue::any_sequence)) {
        crosscheck(
            &format!("(as-max-len? {value} u{max_len})"),
            Ok(Some(Value::some(value.into()).unwrap()))
        )
    }

    #[test]
    fn as_max_len_smaller_than_len_is_none((max_len, value) in (1usize..=16).prop_ind_flat_map2(PropValue::any_sequence)) {
        crosscheck(
            &format!("(as-max-len? {value} u{})", max_len-1),
            Ok(Some(Value::none()))
        )
    }
}
