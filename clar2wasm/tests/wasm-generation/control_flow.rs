use clar2wasm::tools::crosscheck_compare_only;
use clarity::vm::types::TypeSignature;
use proptest::proptest;
use proptest::strategy::Strategy;

use crate::{prop_signature, PropValue};

fn is_optional_type(ty: TypeSignature) -> bool {
    matches!(ty, TypeSignature::OptionalType(_))
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_unwrap_panic(
        vals in prop_signature().
        prop_filter("filter", |ty| is_optional_type(ty.clone()) || ty.is_response_type()).
        prop_flat_map(PropValue::from_type)
    ) {
        crosscheck_compare_only(
            &format!("(unwrap-panic {vals})")
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_unwrap_err_panic(
        vals in prop_signature().
        prop_filter("filter", |ty| ty.is_response_type()).
        prop_flat_map(PropValue::from_type)
    ) {
        crosscheck_compare_only(
            &format!("(unwrap-err-panic {vals})")
        )
    }
}
