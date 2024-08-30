use clar2wasm::tools::crosscheck;
use proptest::prelude::*;

use crate::PropValue;

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

//
// Proptests that should only be executed
// when running Clarity::V2 or Clarity::v3.
//
#[cfg(not(feature = "test-clarity-v1"))]
mod clarity_v2_v3 {
    use clar2wasm::tools::TestEnvironment;
    use clarity::vm::Value;

    use super::*;
    use crate::{prop_signature, type_string, TypePrinter};

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn value_serialized_and_deserialized(val in PropValue::any().prop_filter("Filter condition description", |val| {
            let mut env = TestEnvironment::default();
            env.evaluate(&format!("(to-consensus-buff? {val})")).is_ok()
        })) {
            crosscheck(
                &format!("(from-consensus-buff? {} (unwrap-panic (to-consensus-buff? {})))", val.type_string() ,val),
                Ok(Some(Value::some(val.into()).unwrap()))
            )
        }

        // TODO: see issue #497.
        // The test below should pass when running it in ClarityV1.
        // When issue is fixed this test should be removed from this clarity_v2_v3 module.
        #[test]
        fn data_var_define_and_get(val in PropValue::any()) {
            crosscheck(
                &format!(r##"(define-data-var v {} {val}) (var-get v)"##, val.type_string()),
                Ok(Some(val.into()))
            )
        }

        // TODO: see issue #497.
        // The test below should pass when running it in ClarityV1.
        // When issue is fixed this test should be removed from this clarity_v2_v3 module.
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
}
