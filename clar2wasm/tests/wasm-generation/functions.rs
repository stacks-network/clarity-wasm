use clar2wasm::tools::crosscheck;
use clarity::vm::types::ResponseData;
use clarity::vm::Value;
use proptest::prelude::prop;
use proptest::proptest;
use proptest::strategy::Strategy;

use crate::{PropValue, TypePrinter};

fn generate_random_function_arguments(
    maximum_arguments: usize,
) -> impl Strategy<Value = (String, String)> {
    prop::collection::vec(PropValue::any(), 1..=maximum_arguments).prop_map(|values| {
        let mut arguments = String::from("");
        let mut parameters = String::from("");

        for (i, v) in values.iter().enumerate() {
            arguments.push_str(&format!("(a{i} {}) ", v.type_string()));
            parameters.push_str(&format!("{} ", v));
        }

        (arguments, parameters)
    })
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_private(
        (arguments, parameters) in generate_random_function_arguments(20)
    ) {
            let snippet=&format!("(define-private (func {arguments}) 1) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(Value::Int(1)))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_read_only(
        (arguments, parameters) in generate_random_function_arguments(20)
    ) {
            let snippet=&format!("(define-read-only (func {arguments}) 1) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(Value::Int(1)))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_public_ok(
        (arguments, parameters) in generate_random_function_arguments(20)
    ) {
            let snippet=&format!("(define-public (func {arguments}) (ok 1)) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(Value::Response(ResponseData { committed: true, data: Box::new(Value::Int(1)) })))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_public_err(
        (arguments, parameters) in generate_random_function_arguments(20)
    ) {
            let snippet=&format!("(define-public (func {arguments}) (err 1)) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(Value::Response(ResponseData { committed: false, data: Box::new(Value::Int(1)) })))
            );
    }
}
