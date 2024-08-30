use clar2wasm::tools::crosscheck_with_events;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn print_any(val in PropValue::any()) {
        crosscheck_with_events(
            &format!("(print {val})"),
            Ok(Some(val.into()))
        );
    }
}
