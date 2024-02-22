use clar2wasm::tools::crosscheck;
use clarity::vm::types::OptionalData;
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
    fn crosscheck_index_of(
        seq in PropValue::any_sequence(10usize),
        idx in (0usize..10)
    ) {
        let Value::Sequence(seq_data) = seq.clone().into() else { unreachable!() };
        let item = seq_data.clone().element_at(idx).unwrap();
        let first = match seq_data.contains(item.clone()).unwrap() {
            Some(v) => Value::UInt(v.try_into().unwrap()),
            None => Value::none(),
        };

        let snippet = format!("(index-of? {} {})", seq, PropValue(item));

        crosscheck(
            &snippet,
            Ok(Some(
                Value::Optional(
                    OptionalData {data: Some(Box::new(first))}
                )
            ))
        )
    }
}
