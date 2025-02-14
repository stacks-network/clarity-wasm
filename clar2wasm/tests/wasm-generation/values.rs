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
    use clarity::vm::types::{BuffData, SequenceData};
    use clarity::vm::Value;

    use super::*;

    proptest! {
        #[test]
        fn serialize_deserialize_any_value(val in PropValue::any()) {
            let snippet = format!("(to-consensus-buff? {val})");
            const INPUT_TYPE_ERROR: &str = "could not determine the input type for the serialization function";

            // Pre-check to discard test cases where `to-consensus-buff?` evaluation
            // could not determine the type of the input parameter.
            // For instance, `(to-consensus-buff? (list))` where a `NoType` should not be evaluated.
            let mut env = TestEnvironment::default();
            let pre_check = env.evaluate(&snippet);
            prop_assume!(match pre_check {
                Ok(_) => true,
                Err(ref e) if e.to_string().contains(INPUT_TYPE_ERROR) => false,
                _ => true
            });

            // Serialize the PropValue to check it against
            // the `to-consensus-buff?` implementation.
            let serialized_value = Value::Sequence(SequenceData::Buffer(BuffData {
                data: Value::from(val.clone()).serialize_to_vec().unwrap()
            }));

            let res = pre_check.unwrap(); // Safe to unwrap because of the prop_assume!
            let expected = if res.is_none() {
                Ok(Some(Value::none()))
            } else {
                Ok(Some(Value::some(serialized_value.clone()).unwrap()))
            };

            // Crosscheck serialization
            crosscheck(&snippet, expected);

            // Crosscheck deserialization with `from-consensus-buff?`
            crosscheck(
                &format!(
                    "(from-consensus-buff? {} {})",
                    val.type_string(),
                    serialized_value
                ),
                Ok(Some(Value::some(Value::from(val)).unwrap()))
            );
        }
    }
}
