use proptest::proptest;

use crate::{check_against_interpreter, PropValue};

proptest! {
    #[test]
    fn is_some_always_true(val in PropValue::any()) {
        check_against_interpreter(
            &format!(r#"(is-some (some {val}))"#),
            Some(clarity::vm::Value::Bool(true))
        );
    }
}

proptest! {
    #[test]
    fn is_none_always_false(val in PropValue::any()) {
        check_against_interpreter(
            &format!(r#"(is-none (some {val}))"#),
            Some(clarity::vm::Value::Bool(false))
        );
    }
}
