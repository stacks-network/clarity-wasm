use clar2wasm::tools::crosscheck;
use clarity::vm::types::{ResponseData, TupleData, TypeSignature};
use clarity::vm::{ClarityName, Value};
use proptest::prelude::prop;
use proptest::proptest;
use proptest::strategy::{Strategy, ValueTree};

use crate::{prop_signature, PropValue, TypePrinter};

fn generate_random_function_arguments(
    maximum_arguments: usize,
) -> impl Strategy<Value = (String, String, Vec<PropValue>)> {
    prop::collection::vec(PropValue::any(), 1..=maximum_arguments).prop_map(|values| {
        let mut arguments = String::from("");
        let mut parameters = String::from("");

        for (i, v) in values.iter().enumerate() {
            arguments.push_str(&format!("(a{i} {}) ", v.type_string()));
            parameters.push_str(&format!("{} ", v));
        }

        (arguments, parameters, values)
    })
}

fn check_arguments_accessibility(
    maximum_arguments: usize,
) -> impl Strategy<Value = (String, String, String, Value)> {
    generate_random_function_arguments(maximum_arguments).prop_map(
        |(arguments, parameters, values)| {
            // Accessing values in tuple and returning it from function
            let mut return_value_str = String::from("{");
            let mut tuple_data_items = vec![];

            for (i, v) in values.iter().enumerate() {
                return_value_str.push_str(&format!("a{i}: {},", v));
                tuple_data_items.push((ClarityName::from(format!("a{i}").as_str()), v.clone().0));
            }

            let return_value = Value::Tuple(TupleData::from_data(tuple_data_items).unwrap());

            return_value_str.push('}');

            (arguments, return_value_str, parameters, return_value)
        },
    )
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_private(
        (arguments, return_value_str, parameters, return_value) in check_arguments_accessibility(20)
    ) {
            let snippet=&format!("(define-private (func {arguments}) {return_value_str}) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(return_value))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_private_return(
        (arguments, parameters, values) in generate_random_function_arguments(20)
    ) {
            let last_value=values.last().unwrap();
            let snippet=&format!("(define-private (func {arguments}) {last_value}) (func {parameters})", );

            crosscheck(
                snippet,
                Ok(Some(last_value.clone().0))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_read_only(
        (arguments, return_value_str, parameters, return_value) in check_arguments_accessibility(20)
    ) {
            let snippet=&format!("(define-read-only (func {arguments}) {return_value_str}) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(return_value))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_read_only_return(
        (arguments, parameters, values) in generate_random_function_arguments(20)
    ) {
            let last_value=values.last().unwrap();
            let snippet=&format!("(define-read-only (func {arguments}) {last_value}) (func {parameters})", );

            crosscheck(
                snippet,
                Ok(Some(last_value.clone().0))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_public(
        (arguments, return_value_str, parameters, return_value) in check_arguments_accessibility(20)
    ) {
            let snippet=&format!("(define-public (func {arguments}) (ok {return_value_str})) (func {parameters})");

            crosscheck(
                snippet,
                Ok(Some(Value::Response(ResponseData { committed: true, data: Box::new(return_value) })))
            );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_public_return(
        (response_value, arguments, parameters) in (prop_signature(), prop_signature()).prop_flat_map(|(ok_ty, err_ty)| {
            let response_type = TypeSignature::ResponseType(Box::new((ok_ty, err_ty)));
            let response_value = PropValue::from_type(response_type);

            // Combine with the second strategy
            generate_random_function_arguments(20).prop_map(move |(arguments, parameters, _)| {
                // To convert strategy to a PropValue instance
                let response_value = response_value.new_tree(&mut Default::default()).unwrap().current();
                (response_value, arguments, parameters)
            })
        })
    ) {
            let snippet=&format!("(define-private (func {arguments}) {response_value}) (func {parameters})", );

            crosscheck(
                snippet,
                Ok(Some(response_value.0))
            );
    }
}
