use clar2wasm::tools::evaluate;
use proptest::proptest;

use proptest::prelude::ProptestConfig;

use crate::property_value::PropValue;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1, .. ProptestConfig::default()
    })]
    #[test]
    fn any_single_argument_always_true(val in PropValue::any()) {
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
    fn any_argument_is_equal_to_itself(val in PropValue::any()) {
        println!("{val}");
        assert_eq!(
            evaluate(&format!("(is-eq {val} {val})")),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}
