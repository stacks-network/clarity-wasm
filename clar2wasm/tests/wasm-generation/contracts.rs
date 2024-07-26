use clar2wasm::tools::crosscheck_multi_contract;
use clarity::vm::{types::ResponseData, Value};
use proptest::prelude::*;

use crate::{prop_signature, type_string, PropValue};

use std::fmt::Write;

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
}
