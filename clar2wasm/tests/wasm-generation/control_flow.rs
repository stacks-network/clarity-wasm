use clar2wasm::tools::crosscheck;
use clarity::vm::types::{OptionalData, ResponseData};
use clarity::vm::Value;
use proptest::proptest;

use crate::PropValue;

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
    fn begin(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(begin (some {val}) {val})"#),
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn begin_ok(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(begin (some {val}) (ok {val}))"#),
            Ok(Some(Value::Response(ResponseData { committed: true, data: Box::new(val.into()) })))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn begin_err(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(begin (some {val}) (err {val}))"#),
            Ok(Some(Value::Response(ResponseData { committed: false, data: Box::new(val.into()) })))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn begin_some(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(begin (some {val}))"#),
            Ok(Some(Value::Optional(OptionalData { data: Some(Box::new(val.into())) })))
        );
    }
}
