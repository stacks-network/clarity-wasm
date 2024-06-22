use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::prelude::{prop, proptest};

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
    fn begin(values in prop::collection::vec(PropValue::any(), 1..=20)) {
        let mut expressions = String::new();
        let mut is_response_intermediary = false;

        if let Some((last, rest)) = values.split_last() {
            for v in rest {
                if let Value::Response(_) = v.0 {
                    is_response_intermediary = true;
                }

                if !expressions.is_empty() {
                    expressions.push(' ');
                }

                expressions.push_str(&v.to_string());
            }

            if !expressions.is_empty() {
                expressions.push(' ');
            }
            expressions.push_str(&last.to_string());

            let expr = format!("(begin {})", expressions);

            let expected_val: Result<Option<Value>, ()> = if is_response_intermediary {
                Err(())
            } else {
                Ok(Some(last.clone().into()))
            };

            crosscheck(&expr, expected_val);
        }
    }
}
