use clar2wasm::tools::{crosscheck, crosscheck_compare_only};
use clarity::vm::errors::{Error, ShortReturnType};
use clarity::vm::types::{ListTypeData, SequenceData, SequenceSubtype, TypeSignature};
use clarity::vm::Value;
use proptest::prelude::any;
use proptest::proptest;
use proptest::strategy::{Just, Strategy};

use crate::{bool, prop_signature, type_string, PropValue};

proptest! {
    #![proptest_config(super::runtime_config())]
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
    #![proptest_config(super::runtime_config())]
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
    #![proptest_config(super::runtime_config())]
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
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_optional_some(val in PropValue::any()) {
        let snippet = format!(r#"(unwrap! (some {val}) none)"#);

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[ignore = "see issue: #385"]
    #[test]
    fn unwrap_optional_none(val in PropValue::any()) {
        let snippet = format!(r#"(unwrap! (if true none (some {val})) {val})"#);

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_response_ok(val in PropValue::any()) {
        let snippet = format!(r#"(unwrap! (ok {val}) (err u1))"#);

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[ignore = "see issue: #385"]
    #[test]
    fn unwrap_response_err(val in PropValue::any()) {
        let snippet = format!(r#"(unwrap! (if true (err u1) (ok {val})) {val})"#);

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_optional_none_inside_function(val in PropValue::any()) {
        let snippet = format!("(define-private (foo) (unwrap! (if true none (some {val})) {val})) (foo)");

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_response_err_inside_function(val in PropValue::any()) {
        let snippet = format!("(define-private (foo) (unwrap! (if true (err 1) (ok {val})) {val})) (foo)");

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_err_response_ok_inside_function(val in PropValue::any()) {
        let snippet = format!("(define-private (foo) (unwrap-err! (if true (ok 1) (err {val})) {val})) (foo)");

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_err_response_err_inside_function(val in PropValue::any()) {
        let snippet = format!("(define-private (foo) (unwrap-err! (if false (ok 1) (err {val})) {val})) (foo)");

        crosscheck(
            &snippet,
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_asserts_boolean(bool in any::<bool>(), val in PropValue::any()) {
        let expected = if bool {
            Ok(Some(Value::Bool(bool)))
        } else {
            Err(Error::ShortReturn(ShortReturnType::AssertionFailed(Value::from(val.clone()))))
        };

        crosscheck(
            &format!("(asserts! {bool} {val})"),
            expected
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[ignore = "see issue: #385"]
    #[test]
    fn crosscheck_try_optional_inside_function(bool in bool(), val in PropValue::any()) {
        let expected = match bool.to_string().as_str() {
            "true" => val.clone(),
            "false" => PropValue::from(Value::none()),
            _ => panic!("Invalid boolean string"),
        };

        let snippet = format!("(define-private (foo) (if {bool} (some {val}) none)) (try! (foo))");

        crosscheck(
            &snippet,
            Ok(Some(expected.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[ignore = "see issue: #385"]
    #[test]
    fn crosscheck_try_response_inside_function(
        bool in bool(),
        val in PropValue::any(),
        err_val in PropValue::any()
    ) {
        let expected = match bool.to_string().as_str() {
            "true" => val.clone(),
            "false" => err_val.clone(),
            _ => panic!("Invalid boolean string"),
        };

        let snippet = format!("(define-private (foo) (if {bool} (ok {val}) (err {err_val}))) (try! (foo))");

        crosscheck(
            &snippet,
            Ok(Some(expected.into()))
        );
    }
}

const FILTER_PRELUDE: &str = "
(define-private (is-even? (x int))
        (is-eq (* (/ x 2) 2) x))

(define-private (grob (x (response int int)))
  (match x
    a (is-even? a)
    b (not (is-even? b))))";

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_filter_responses_short(
        seq in PropValue::from_type(
            TypeSignature::SequenceType(
                SequenceSubtype::ListType(
                    ListTypeData::new_list(
                        TypeSignature::ResponseType(
                            Box::new((TypeSignature::IntType, TypeSignature::IntType))),
                        2).unwrap()
                )
            )
        )
    ) {
        if let Value::Sequence(SequenceData::List(ld)) = seq.inner() {
            // Empty sequences fail in interpreter as well
            if !ld.data.is_empty() {
                let snippet = format!("{FILTER_PRELUDE} (filter grob {})", seq);

                crosscheck_compare_only(
                    &snippet,
                );
            }
        }
    }

    #[test]
    fn crosscheck_filter_responses_long(
        seq in PropValue::from_type(
            TypeSignature::SequenceType(
                SequenceSubtype::ListType(
                    ListTypeData::new_list(
                        TypeSignature::ResponseType(
                            Box::new((TypeSignature::IntType, TypeSignature::IntType))),
                        100).unwrap()
                )
            )
        )
    ) {
        if let Value::Sequence(SequenceData::List(ld)) = seq.inner() {
            // Empty sequences fail in interpreter as well
            if !ld.data.is_empty() {
                let snippet = format!("{FILTER_PRELUDE} (filter grob {})", seq);

                crosscheck_compare_only(
                    &snippet,
                );
            }
        }
    }
}

//
// Proptests that should only be executed
// when running Clarity::V2 or Clarity::v3.
//
#[cfg(not(feature = "test-clarity-v1"))]
mod clarity_v2_v3 {
    use super::*;
    use crate::runtime_config;

    proptest! {
        #![proptest_config(runtime_config())]

        // TODO: see issue #497.
        // The test below should pass when running it in ClarityV1.
        // When issue is fixed this test should be removed from this clarity_v2_v3 module.
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

        // TODO: see issue #497.
        // The test below should pass when running it in ClarityV1.
        // When issue is fixed this test should be removed from this clarity_v2_v3 module.
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
}
