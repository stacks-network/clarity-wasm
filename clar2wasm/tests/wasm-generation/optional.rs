use clar2wasm::tools::crosscheck;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #[ignore]
    #[test]
    fn is_some_always_true(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-some (some {val}))"#),
            Ok(Some(clarity::vm::Value::Bool(true)))
        );
    }
}

proptest! {
    #[ignore]
    #[test]
    fn is_none_always_false(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-none (some {val}))"#),
            Ok(Some(clarity::vm::Value::Bool(false)))
        );
    }
}
