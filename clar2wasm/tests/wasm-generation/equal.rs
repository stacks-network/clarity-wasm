use clar2wasm::tools::evaluate;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #[test]
    fn is_eq_one_argument_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-eq {val})"#)).unwrap(),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}
proptest! {
    #[test]
    fn is_eq_value_with_itself_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-eq {val} {val})"#)).unwrap(),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}

proptest! {
    #[test]
    fn is_eq_value_with_itself_always_true_3(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-eq {val} {val} {val})"#)).unwrap(),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}
