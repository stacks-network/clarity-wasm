use proptest::{prop_compose, proptest};

use crate::{check_against_interpreter, prop_signature, PropValue};

proptest! {
    #[test]
    fn default_to_with_none_is_always_default(val in PropValue::any()) {
        check_against_interpreter(&format!(r#"(default-to {val} none)"#), Some(val.into()));
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
        check_against_interpreter(
            &format!(r#"(default-to {default} (some {value}))"#),
            Some(value.into())
        );
    }
}
