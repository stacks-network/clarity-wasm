use clar2wasm::tools::evaluate;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #[test]
    fn is_some_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-some (some {val}))"#)),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}

proptest! {
    #[test]
    fn is_none_always_false(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-none (some {val}))"#)),
            Some(clarity::vm::Value::Bool(false))
        )
    }
}
