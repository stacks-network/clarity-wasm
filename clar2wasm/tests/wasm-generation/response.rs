use clar2wasm::tools::evaluate;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #[test]
    fn is_ok_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-ok (ok {val}))"#)),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}

proptest! {
    #[test]
    fn is_ok_always_false(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-ok (err {val}))"#)),
            Some(clarity::vm::Value::Bool(false))
        )
    }
}

proptest! {
    #[test]
    fn is_err_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-err (err {val}))"#)),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}

proptest! {
    #[test]
    fn is_err_always_false(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-err (ok {val}))"#)),
            Some(clarity::vm::Value::Bool(false))
        )
    }
}
