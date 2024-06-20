use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::proptest;
use proptest::strategy::Strategy;

use crate::{random_expressions, PropValue, TypePrinter};

const DEFINE_PRIVATE_READONLY: [&str; 2] = ["define-private", "define-read-only"];

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_private_readonly(
        (v, (expr, expected_val, is_response_intermediary)) in PropValue::any().prop_flat_map(|v| {
            random_expressions(20,false).prop_map(move |tuple| (v.clone(), tuple))
        })
) {
        for function in DEFINE_PRIVATE_READONLY{
            let expr=format!("(begin {expr})");
            let snippet=&format!(r#"({function} (func (a {})) {expr}) (func {v})"#,v.type_string());

            let expected_val: Result<Option<Value>,()>=if is_response_intermediary{
                Err(())
            }else{
                Ok(Some(expected_val.clone().into()))
            };

            crosscheck(
                snippet,
                expected_val
            );
        }
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_public(
        (v, (expr, expected_val, is_response_intermediary)) in PropValue::any().prop_flat_map(|v| {
            random_expressions(10,true).prop_map(move |tuple| (v.clone(), tuple))
        })
) {
            let expr=format!("(begin {expr})");
            let snippet=&format!(r#"(define-public (func (a {})) {expr}) (func {v})"#,v.type_string());

            let expected_val: Result<Option<Value>,()>=if is_response_intermediary{
                Err(())
            }else{
                Ok(Some(expected_val.clone().into()))
            };

            crosscheck(
                snippet,
                expected_val
            );
    }
}
