use clar2wasm::tools::crosscheck;
use proptest::{prop_compose, proptest};

use crate::{prop_signature, PropValue};

proptest! {
    #[test]
    fn default_to_with_none_is_always_default(val in PropValue::any()) {
        crosscheck(&format!(r#"(default-to {val} none)"#), Ok(Some(val.into())));
    }
}

prop_compose! {
    fn default_and_value_of_same_type()
        (signature in prop_signature())
        (default in PropValue::from_type(signature.clone()) , value in PropValue::from_type(signature))
        -> (PropValue, PropValue) {
            (default, value)
        }
}

proptest! {
    #[test]
    fn default_to_with_some_is_always_value((default, value) in default_and_value_of_same_type()) {
        crosscheck(
            &format!(r#"(default-to {default} (some {value}))"#),
            Ok(Some(value.into()))
        );
    }
}
