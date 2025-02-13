use clar2wasm::tools::crosscheck;
use proptest::prelude::*;

use crate::{prop_signature, type_string, PropValue, TypePrinter as _};

proptest! {
    #![proptest_config(super::runtime_config())]
    #[test]
    fn evaluated_value_is_the_value_itself(val in PropValue::any()) {
        crosscheck(
            &val.to_string(),
            Ok(Some(val.into()))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]
    #[test]
    fn constant_define_and_get(val in PropValue::any()) {
        crosscheck(
            &format!(r#"(define-constant cst {val}) cst"#),
            Ok(Some(val.into()))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn data_var_define_and_get(val in PropValue::any()) {
        crosscheck(
            &format!(r##"(define-data-var v {} {val}) (var-get v)"##, val.type_string()),
            Ok(Some(val.into()))
        )
    }

    #[test]
    fn data_var_define_set_and_get(
        (ty, v1, v2) in prop_signature()
            .prop_flat_map(|ty| {
                (Just(ty.clone()), PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!(r#"(define-data-var v {} {v1}) (var-set v {v2}) (var-get v)"#, type_string(&ty)),
            Ok(Some(v2.into()))
        )
    }
}

//
// Proptests that should only be executed
// when running Clarity::V2 or Clarity::v3.
//
#[cfg(not(feature = "test-clarity-v1"))]
mod clarity_v2_v3 {
    use clar2wasm::tools::TestEnvironment;
    use clarity::vm::Value;
    use clarity::vm::errors::{CheckErrors, Error, WasmError};

    use super::*;

    proptest! {
        #[test]
        fn value_serialized_and_deserialized(val in PropValue::any()) {
            let mut env = TestEnvironment::default();

            let to_consensus_snippet = format!("(to-consensus-buff? {val})");
            let pre_check = env.evaluate(&to_consensus_snippet);

            // Discard test cases only when `to-consensus-buff?` evaluation
            // could not determine the type of the input parameter.
            // For instance, `(to-consensus-buff? (list))` where a `NoType` should not be evaluated.
            let err_msg = "could not determine the input type for the serialization function";
            prop_assume!(match pre_check {
                Ok(_) => true,
                Err(ref e) if e.to_string().contains(err_msg) => false,
                _ => true
            });

            let snippet = format!(
                "(from-consensus-buff? {} (unwrap-panic (to-consensus-buff? {})))",
                val.type_string(),
                val
            );

            let res = pre_check.unwrap(); // Safe to unwrap because of the prop_assume!
            let expected = if res.is_none() {
                Ok(Some(Value::none()))
            } else {
                Ok(Some(Value::some(val.into()).unwrap()))
            };

            crosscheck(&snippet, expected);
        }
    }
}
