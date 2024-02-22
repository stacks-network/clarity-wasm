use clar2wasm::tools::{crosscheck, crosscheck_compare_only};
use clarity::vm::Value;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_eq_one_argument_always_true(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-eq {val})"#),
            Ok(Some(clarity::vm::Value::Bool(true)))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_eq_value_with_itself_always_true(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-eq {val} {val})"#),
            Ok(Some(clarity::vm::Value::Bool(true)))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_eq_value_with_itself_always_true_3(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-eq {val} {val} {val})"#),
            Ok(Some(clarity::vm::Value::Bool(true)))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_index_of(seq_data in PropValue::any_sequence(20usize), idx in (0..20usize)) {
        let seq = seq_data.clone();
        let snippet = format!("(element-at? {seq} u{idx})");

        let expected = {
            let Value::Sequence(seq) = seq.clone().into() else { unreachable!() };
            seq.element_at(idx).map_or_else(Value::none, |v| Value::some(v).unwrap())
        };

        crosscheck(&format!("(index-of? {seq} (try! {snippet})"), Ok(Some(expected)));
    }
}
