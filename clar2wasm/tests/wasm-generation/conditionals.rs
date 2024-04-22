use clar2wasm::tools::{crosscheck, crosscheck_compare_only};
use clarity::vm::types::{ASCIIData, CharType, SequenceData};
use clarity::vm::Value;
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

    #[ignore = "ignored until issue #104 is resolved"]
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

    #[ignore = "ignored until issue #104 is resolved"]
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

    #[ignore = "ignored until issue #104 is resolved"]
    #[test]
    fn crosscheck_asserts_true(bool in bool(), val in PropValue::any()) {
        let expected = match bool.to_string().as_str() {
            "true" => PropValue::from(bool.clone()),
            "false" => val.clone(),
            _ => panic!("Invalid boolean string"),
        };

        crosscheck(
            &format!("(asserts! {bool} {val})"),
            Ok(Some(expected.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[ignore = "ignored until issue #104 is resolved"]
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

    #[ignore = "ignored until issue #104 is resolved"]
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

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crosscheck_filter(
        seq in PropValue::any_sequence(3usize)
    ) {
        let Value::Sequence(seq_data) = seq.clone().into() else { unreachable!() };

        match seq_data {
            clarity::vm::types::SequenceData::Buffer(_) => {
                let snippet = format!("(define-private (only-zero (byte (buff 1))) (is-eq byte 0x00)) (buff-to-int-be (filter only-zero {seq}))");

                crosscheck(
                    &snippet,
                    Ok(Some(Value::Int(0)))
                )
            },
            clarity::vm::types::SequenceData::List(data) => {
                let max_len = data.type_signature.get_max_len();
                let item_ty = data.type_signature.get_list_item_type().to_string();

                crosscheck_compare_only(
                    &format!("(define-private (foo (el (list {max_len} {item_ty})))
                        (not (is-eq u0 (len el))))
                        (filter foo (list {seq}))"
                    )
                );
            },
            clarity::vm::types::SequenceData::String(data) => {
                let data = data.to_string();
                let v: Vec<u32> = data.as_str().chars().filter_map(|a| a.to_digit(10)).collect();
                let expected: String = v.into_iter().map(|i| i.to_string()).collect::<String>();
                let snippet = format!("(define-private (is-int (char (string-ascii 1)))
                    (is-eq 0 (* 0 (unwrap! (string-to-int? char) false))))
                    (filter is-int {seq})"
                );

                crosscheck(
                    &snippet,
                    Ok(Some(Value::Sequence(SequenceData::String(
                        CharType::ASCII(ASCIIData {
                            data: expected.bytes().collect(),
                        }),
                    )))),
                )
            },
        };
    }
}
