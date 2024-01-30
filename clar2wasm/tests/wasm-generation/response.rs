use clar2wasm::tools::crosscheck;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_ok_always_true(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-ok (ok {val}))"#),
            Ok(Some(clarity::vm::Value::Bool(true)))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_ok_always_false(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-ok (err {val}))"#),
            Ok(Some(clarity::vm::Value::Bool(false)))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_err_always_true(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-err (err {val}))"#),
            Ok(Some(clarity::vm::Value::Bool(true)))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn is_err_always_false(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(is-err (ok {val}))"#),
            Ok(Some(clarity::vm::Value::Bool(false)))
        )
    }
}
