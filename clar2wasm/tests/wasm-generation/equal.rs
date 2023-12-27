use clar2wasm::tools::evaluate;
use proptest::proptest;

use crate::PropValue;

use proptest::prelude::ProptestConfig;

proptest! {
    #[test]
    fn is_eq_one_argument_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-eq {val})"#)),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}
proptest! {
    #[test]
    fn is_eq_value_with_itself_always_true(val in PropValue::any()) {
        assert_eq!(
            evaluate(&format!(r#"(is-eq {val} {val})"#)),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1,
        .. ProptestConfig::default()
    })]
    #[test]
    fn is_eq_value_with_itself_always_true_3(val in PropValue::any()) {
        dbg!(&val);
        assert_eq!(
            evaluate(&format!(r#"(is-eq {val} {val} {val})"#)),
            Some(clarity::vm::Value::Bool(true))
        )
    }
}
