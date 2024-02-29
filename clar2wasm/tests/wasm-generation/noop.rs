use clar2wasm::tools::crosscheck;
use clarity::vm::Value;
use proptest::arbitrary::any;
use proptest::proptest;
use proptest::strategy::Strategy;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_noop_to_uint(
        val in any::<i128>()
        .prop_filter("non-negative", |v| v >= &0i128)
    ) {
        crosscheck(
            &format!("(to-uint {})", PropValue(Value::Int(val))),
            Ok(Some(Value::UInt(val.try_into().unwrap())))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_noop_to_uint_err(
        val in any::<i128>()
        .prop_filter("negative", |v| v < &0i128)
    ) {
        crosscheck(
            &format!("(to-uint {})", PropValue(Value::Int(val))),
            Err(())
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_noop_to_int(
        val in any::<u128>()
        .prop_filter("range", |v| v < &2u128.pow(126))) {
        crosscheck(
            &format!("(to-int {})", PropValue(Value::UInt(val))),
            Ok(Some(Value::Int(val.try_into().unwrap())))
        )
    }
}
