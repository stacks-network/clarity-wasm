use clar2wasm::tools::crosscheck;
use proptest::proptest;
use proptest::strategy::{Just, Strategy};

use crate::{prop_signature, type_string, PropValue};

proptest! {
    #[ignore]
    #[test]
    fn if_true_returns_first_value(
        (v1, v2) in prop_signature()
            .prop_flat_map(|ty| {
                (PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!(r#"(if true {v1} {v2})"#),
            Ok(Some(v1.into()))
        )
    }
}

proptest! {
    #[ignore]
    #[test]
    fn if_false_returns_second_value(
        (v1, v2) in prop_signature()
            .prop_flat_map(|ty| {
                (PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!(r#"(if false {v1} {v2})"#),
            Ok(Some(v2.into()))
        )
    }
}

proptest! {
    #[ignore]
    #[test]
    fn match_optional_some(
        (initial, some_val, none_val) in (prop_signature(), prop_signature())
            .prop_flat_map(|(original_ty, ty)| {
                (PropValue::from_type(original_ty), PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!("(match (some {initial}) value {some_val} {none_val})"),
            Ok(Some(some_val.into()))
        )
    }

    #[ignore]
    #[test]
    fn match_optional_none(
        (original_ty, some_val, none_val) in (prop_signature(), prop_signature())
            .prop_flat_map(|(original_ty, ty)| {
                (Just(original_ty), PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!(r#"
                (define-data-var null (optional {}) none)
                (match (var-get null) value {some_val} {none_val})
            "#, type_string(&original_ty)),
            Ok(Some(none_val.into()))
        )
    }

    #[ignore]
    #[test]
    fn match_response_ok(
        (original_ok_ty, original_ok_val, original_err_ty, ok_val, err_val) in (prop_signature(), prop_signature(), prop_signature())
            .prop_flat_map(|(original_ok_ty, original_err_ty, ty)| {
                (Just(original_ok_ty.clone()), PropValue::from_type(original_ok_ty), Just(original_err_ty), PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!(r#"
                (define-data-var okval (response {} {}) (ok {original_ok_val}))
                (match (var-get okval) value {ok_val} err-value {err_val})
            "#, type_string(&original_ok_ty), type_string(&original_err_ty)),
            Ok(Some(ok_val.into()))
        )
    }
    #[ignore]
    #[test]
    fn match_response_err(
        (original_ok_ty, original_err_ty, original_err_val, ok_val, err_val) in (prop_signature(), prop_signature(), prop_signature())
            .prop_flat_map(|(original_ok_ty, original_err_ty, ty)| {
                (Just(original_ok_ty), Just(original_err_ty.clone()), PropValue::from_type(original_err_ty), PropValue::from_type(ty.clone()), PropValue::from_type(ty))
            })
        )
    {
        crosscheck(
            &format!(r#"
                (define-data-var errval (response {} {}) (err {original_err_val}))
                (match (var-get errval) value {ok_val} err-value {err_val})
            "#, type_string(&original_ok_ty), type_string(&original_err_ty)),
            Ok(Some(err_val.into()))
        )
    }
}
