use clar2wasm::tools::evaluate;
use proptest::proptest;

use proptest::prelude::ProptestConfig;

use crate::PropValue;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 500,
        .. ProptestConfig::default()
    })]
    #[test]
    fn evaluated_value_is_the_value_itself(val in PropValue::any()) {
        assert_eq!(
            evaluate(&val.to_string()),
            Some(val.into())
        )
    }
}
