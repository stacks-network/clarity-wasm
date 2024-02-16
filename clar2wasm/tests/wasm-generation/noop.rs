use clar2wasm::tools::crosscheck_compare_only;
use proptest::proptest;
use proptest::strategy::Strategy;

use crate::{int, uint, PropValue};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_noop_to_int(val in uint().prop_map(PropValue)) {
        crosscheck_compare_only(
            &format!("(to-int {val})")
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_noop_to_uint(val in int().prop_map(PropValue)) {
        crosscheck_compare_only(
            &format!("(to-uint {val})")
        )
    }
}
