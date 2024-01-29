use clar2wasm::tools::crosscheck;
use clarity::vm::{types::TypeSignature, Value};
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

proptest! {
    #[test]
    fn concat_crosscheck((seq1, seq2) in (0usize..=16).prop_flat_map(PropValue::any_sequence).prop_ind_flat_map2(|seq1| PropValue::from_type(dbg!(TypeSignature::type_of(&seq1.into()))))) {
        let snippet = dbg!(format!("(concat {seq1} {seq2})"));

        let expected = {
            let Value::Sequence(mut seq_data1) = seq1.into() else { unreachable!() };
            let Value::Sequence(seq_data2) = seq2.into() else { unreachable!() };
            seq_data1.concat(&clarity::types::StacksEpochId::latest(), seq_data2).expect("Unable to concat sequences");
            Value::Sequence(seq_data1)
        };

        crosscheck(&snippet, Ok(Some(expected)));
    }
}
