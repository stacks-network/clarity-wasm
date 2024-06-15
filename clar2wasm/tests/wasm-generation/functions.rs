use clar2wasm::tools::crosscheck;
use clarity::vm::{types::ResponseData, Value};
use proptest::proptest;

use crate::{PropValue, TypePrinter};

const NO_RESPONSE_FUNCTIONS: [&str; 2] = ["define-private", "define-read-only"];

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_no_response_functions(
        v in PropValue::any()
) {
        for function in NO_RESPONSE_FUNCTIONS{
            let snippet=&format!(r#"({function} (func (a {})) 1) (func {v})"#,v.type_string());
            
            crosscheck(
                snippet,
                Ok(Some(Value::Int(1)))
            );
        }

    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_public(
        v in PropValue::any()
) {
            let snippet=&format!(r#"(define-public (func (a {})) (ok 1)) (func {v})"#,v.type_string());

            crosscheck(
                snippet,
                Ok(Some(Value::Response(ResponseData{committed:true, data: Box::new(Value::Int(1))})))
            );
    }
}
