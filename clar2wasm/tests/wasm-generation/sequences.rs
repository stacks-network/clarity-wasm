use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::prelude::*;

use crate::{prop_signature, PropValue};

proptest! {
    #![proptest_config(super::runtime_config())]
    #[test]
    fn append_value_to_list(mut values in (prop_signature(), 1usize..32).prop_flat_map(|(ty, size)| PropValue::many_from_type(ty, size))) {
        let expected = Value::cons_list_unsanitized(values.iter().cloned().map(Value::from).collect()).unwrap();

        let elem = values.pop().unwrap();
        let values = PropValue::try_from(values).unwrap();

        crosscheck(&format!("(append {values} {elem})"), Ok(Some(expected)))
    }
}
