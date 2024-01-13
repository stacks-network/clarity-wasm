use proptest::proptest;

use crate::{check_against_interpreter, PropValue};

proptest! {
    #[test]
    fn is_eq_one_argument_always_true(val in PropValue::any()) {
        check_against_interpreter(
            &format!(r#"(is-eq {val})"#),
            Some(clarity::vm::Value::Bool(true))
        );
    }
}

proptest! {
    #[test]
    fn is_eq_value_with_itself_always_true(val in PropValue::any()) {
        check_against_interpreter(
            &format!(r#"(is-eq {val} {val})"#),
            Some(clarity::vm::Value::Bool(true))
        );
    }
}

proptest! {
    #[test]
    fn is_eq_value_with_itself_always_true_3(val in PropValue::any()) {
        check_against_interpreter(
            &format!(r#"(is-eq {val} {val} {val})"#),
            Some(clarity::vm::Value::Bool(true))
        );
    }
}
