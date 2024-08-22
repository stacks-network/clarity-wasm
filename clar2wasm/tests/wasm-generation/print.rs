use clar2wasm::tools::crosscheck;
use proptest::proptest;

use crate::PropValue;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn print_any(val in PropValue::any()) {
        crosscheck(
            &format!("(print {val})"),
            Ok(Some(val.into()))
        );
    }
}
