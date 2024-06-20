use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::proptest;
use crate::{random_expressions, PropValue};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_panic_optional(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(unwrap-panic (some {val}))"#),
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_panic_response_ok(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(unwrap-panic (ok {val}))"#),
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_panic_response_err(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(unwrap-panic (err {val}))"#),
            Err(())
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_err_panic(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(unwrap-err-panic (err {val}))"#),
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_err_panic_ok(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(unwrap-err-panic (ok {val}))"#),
            Err(())
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn begin((expr, expected_val, is_response_intermediary) in random_expressions(20,false)) {
        let expr=format!("(begin {})", expr);
        let expected_val:Result<Option<Value>, ()> = if is_response_intermediary{
            Err(())
        } else{
            Ok(Some(expected_val.into()))
        };

        crosscheck(
            &expr,
            expected_val
        );

    }
}
