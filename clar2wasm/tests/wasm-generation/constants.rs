use clar2wasm::tools::crosscheck;
use clar2wasm::wasm_utils::signature_from_string;
use clarity::types::StacksEpochId;
use clarity::vm::ClarityVersion;
use proptest::prelude::*;

use crate::{
    int, qualified_principal, standard_principal, string_ascii, string_utf8, uint, PropValue,
};

fn literal() -> impl Strategy<Value = PropValue> {
    prop_oneof![
        int(),
        uint(),
        standard_principal(),
        qualified_principal(),
        (0..32u32).prop_flat_map(string_ascii),
        (0..32u32).prop_flat_map(string_utf8)
    ]
    .prop_map_into()
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn define_constant_from_literal(lit in literal()) {
        crosscheck(&format!("(define-constant cst {lit}) cst"), Ok(Some(lit.into())));
    }

    #[test]
    fn define_constant_from_anything(val in PropValue::any()) {
        crosscheck(&format!("(define-constant cst {val}) cst"), Ok(Some(val.into())));
    }

    #[test]
    fn define_constant_from_non_literal(val in PropValue::any()) {
        let snippet = format!(r#"
            (define-private (foo) {val})
            (define-constant cst (foo)) cst
        "#);
        crosscheck(&snippet, Ok(Some(val.into())));
    }

    #[test]
    fn define_constant_from_large_complex(val in PropValue::from_type(signature_from_string("(list 18 (list 31 (string-ascii 105)))", ClarityVersion::latest(), StacksEpochId::latest()).unwrap())) {
        crosscheck(
            &format!("(define-constant cst {val}) cst"),
            Ok(Some(val.into())),
        )
    }
}
