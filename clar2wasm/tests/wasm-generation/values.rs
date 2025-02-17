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
