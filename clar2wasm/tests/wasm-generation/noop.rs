use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::proptest;
use proptest::strategy::Strategy;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_noop_to_uint(val in (0u128..=126)
        .prop_map(|exp| 2u128.pow(exp.try_into().unwrap()))
        .prop_map(Value::UInt)
    ) {
        crosscheck(
            &format!("(to-uint (to-int {val}))"),
            Ok(Some(val))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_noop_to_int(val in (0u128..=126)
        .prop_map(|exp| 2i128.pow(exp.try_into().unwrap()))
        .prop_map(Value::Int)
    ) {
        crosscheck(
            &format!("(to-int (to-uint {val}))"),
            Ok(Some(val))
        )
    }
}
