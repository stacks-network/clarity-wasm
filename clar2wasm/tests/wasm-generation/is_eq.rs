use clar2wasm::tools::evaluate;
use proptest::proptest;

use crate::property_value::any_value;

use proptest::prelude::ProptestConfig;

proptest! {
    #[test]
    fn any_single_argument_always_true(val in any_value()) {
        assert_eq!(
            evaluate(&format!("(is-eq {val})")),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1, .. ProptestConfig::default()
    })]
    #[test]
    fn any_argument_is_equal_to_itself(val in any_value()) {
        println!("{val}");
        assert_eq!(
            evaluate(&format!("(is-eq {val} {val})")),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}
