use std::fmt::Write;

use clar2wasm::tools::{crosscheck, crosscheck_multi_contract};
use clarity::vm::types::{ResponseData, TupleData};
use clarity::vm::{ClarityName, Value};
use proptest::prelude::*;

use crate::{prop_signature, type_string, PropValue};

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn contract_call_accepts_any_args(
        (tys, values)
            in prop::collection::vec(
                prop_signature().prop_ind_flat_map2(|ty| PropValue::from_type(ty.clone())),
                1..=20
            )
            .prop_map(|arg_ty| arg_ty.into_iter().unzip::<_, _, Vec<_>, Vec<_>>())
            .no_shrink(),
        result in PropValue::any().no_shrink()
    ) {
        // first contract
        let first_contract_name = "foo".into();
        let mut function_arguments = String::new();
        for (name, ty) in ('a'..).zip(tys.iter()) {
            write!(function_arguments, "({name} {}) ", type_string(ty)).unwrap();
        }
        let first_snippet = format!(
            r#"
                (define-public (foofun {function_arguments})
                    (ok {result})
                )
            "#
        );

        // second contract
        let second_contract_name = "bar".into();
        let mut call_arguments = String::new();
        for value in values {
            write!(call_arguments, "{value} ").unwrap();
        }
        let second_snippet =
            format!(r#"(contract-call? .{first_contract_name} foofun {call_arguments})"#);


        crosscheck_multi_contract(
            &[
                (first_contract_name, &first_snippet),
                (second_contract_name, &second_snippet),
            ],
            Ok(Some(Value::Response(ResponseData {
                committed: true,
                data: Box::new(result.into()),
            }))),
        );
    }

    #[test]
    fn contract_call_returns_any_value_from_argument(
        (ty, value) in prop_signature().prop_ind_flat_map2(|ty| PropValue::from_type(ty.clone())).no_shrink()
    ) {
        // first contract
        let first_contract_name = "foo".into();
        let first_snippet = format!(
            r#"
                (define-public (foofun (a {}))
                    (ok a)
                )
            "#, type_string(&ty)
        );

        // second contract
        let second_contract_name = "bar".into();
        let second_snippet =
            format!(r#"(contract-call? .{first_contract_name} foofun {value})"#);

        crosscheck_multi_contract(
            &[
                (first_contract_name, &first_snippet),
                (second_contract_name, &second_snippet),
            ],
            Ok(Some(Value::Response(ResponseData {
                committed: true,
                data: Box::new(value.into()),
            }))),
        );
    }

    #[test]
    fn contract_call_can_use_all_arguments(
        (tys, values)
            in prop::collection::vec(
                prop_signature()
                    .prop_ind_flat_map2(|ty| PropValue::from_type(ty.clone())),
                1..=20
            )
            .prop_map(|arg_ty| arg_ty.into_iter().unzip::<_, _, Vec<_>, Vec<_>>())
            .no_shrink(),
    ) {
        let first_contract_name = "foo".into();
        let mut function_arguments = String::new();
        for (name, ty) in ('a'..).zip(tys.iter()) {
            write!(function_arguments, "({name} {}) ", type_string(ty)).unwrap();
        }
        let expected_res = ('a'..)
            .take(tys.len())
            .fold(String::new(), |mut output, arg| {
                write!(output, "{arg}: {arg}, ").unwrap();
                output
            });
        let first_snippet = format!(
            r#"
                (define-public (foofun {function_arguments})
                    (ok {{ {expected_res} }})
                )
            "#
        );

        // second contract
        let second_contract_name = "bar".into();
        let mut call_arguments = String::new();
        for value in values.iter() {
            write!(call_arguments, "{value} ").unwrap();
        }
        let second_snippet =
            format!(r#"(contract-call? .{first_contract_name} foofun {call_arguments})"#);

        let expected = TupleData::from_data(
            ('a'..)
                .map(|c| ClarityName::try_from(c.to_string()).unwrap())
                .zip(values.into_iter().map(Value::from))
                .collect(),
        )
        .unwrap()
        .into();

        crosscheck_multi_contract(
            &[
                (first_contract_name, &first_snippet),
                (second_contract_name, &second_snippet),
            ],
            Ok(Some(Value::Response(ResponseData {
                committed: true,
                data: Box::new(expected),
            }))),
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn as_contract_can_return_any_value(
        value in PropValue::any()
    ) {
        crosscheck(&format!("(as-contract {value})"), Ok(Some(value.into())));
    }
}
