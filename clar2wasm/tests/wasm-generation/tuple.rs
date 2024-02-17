use clar2wasm::tools::crosscheck_compare_only;
use proptest::proptest;
use proptest::strategy::Strategy;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_tuple(
        key in "[a-zA-Z]{1}([0-9][a-zA-Z]|[a-zA-Z][0-9]){1,20}".prop_map(|k| k),
        expr_1 in (1..10usize).prop_flat_map(PropValue::any_sequence))
    {
        crosscheck_compare_only(
            &format!("(tuple ({key} {expr_1}))")
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_merge(
        key in "[a-zA-Z]{1}([0-9][a-zA-Z]|[a-zA-Z][0-9]){1,20}".prop_map(|k| k),
        (expr_1, expr_2) in (1..10usize).prop_flat_map(|v| (PropValue::any_sequence(v), PropValue::any_sequence(v))))
    {
        crosscheck_compare_only(
            &format!("(merge (tuple ({key} {expr_1})) (tuple ({key}_ {expr_2})))")
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_get(
        key in "[a-zA-Z]{1}([0-9][a-zA-Z]|[a-zA-Z][0-9]){1,20}".prop_map(|k| k),
        expr_1 in (1..10usize).prop_flat_map(PropValue::any_sequence))
    {
        crosscheck_compare_only(
            &format!("(get {key} (tuple ({key} {expr_1})))")
        )
    }
}
