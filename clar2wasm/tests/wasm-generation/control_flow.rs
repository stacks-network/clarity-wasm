use clar2wasm::tools::crosscheck;
use clarity::vm::types::ResponseData;
use clarity::vm::Value;
use proptest::prelude::prop;
use proptest::proptest;
use proptest::strategy::Strategy;

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

fn unwrap_response_expression(value: PropValue) -> String {
    match value.clone().0 {
        Value::Response(ResponseData {
            committed: false,
            data,
        }) => match *data {
            Value::Response(_) => unwrap_response_expression(PropValue::from(*data)),
            _ => format!("(unwrap-err! {} false)", value),
        },
        Value::Response(ResponseData {
            committed: true,
            data,
        }) => match *data {
            Value::Response(_) => unwrap_response_expression(PropValue::from(*data)),
            _ => format!("(unwrap! {} (err 1))", value),
        },
        _ => {
            format!("{}", value)
        }
    }
}

fn begin_strategy() -> impl Strategy<Value = (String, PropValue)> {
    prop::collection::vec(PropValue::any(), 1..=100).prop_map(|values| {
        let mut expressions = String::new();
        let len = values.len();

        for (i, v) in values.iter().enumerate() {
            let s = if i == len - 1 {
                format!("{}", v)
            } else {
                unwrap_response_expression(v.clone())
            };

            if !expressions.is_empty() {
                expressions.push(' ');
            }

            expressions.push_str(&s);
        }

        let last_value = values.last().unwrap().clone();

        (format!("(begin {})", expressions), last_value)
    })
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
    fn begin((expr, expected_val) in begin_strategy()) {
        crosscheck(
            &expr,
            Ok(Some(expected_val.into()))
        );
    }
}
