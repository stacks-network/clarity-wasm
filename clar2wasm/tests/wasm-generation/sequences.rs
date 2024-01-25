use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::proptest;
use proptest::strategy::Strategy as _;

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
