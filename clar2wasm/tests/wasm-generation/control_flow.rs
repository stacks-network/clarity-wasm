use clar2wasm::tools::crosscheck;
use clarity::vm::types::{SequenceSubtype, StringSubtype, TypeSignature};
use proptest::strategy::{Just, Strategy};
use proptest::{prop_oneof, proptest};

use crate::PropValue;

fn strategies_for_control_flow() -> impl Strategy<Value = TypeSignature> {
    prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        Just(TypeSignature::BoolType),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
        ))),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::StringType(
            StringSubtype::ASCII(s.try_into().unwrap())
        ))),
        Just(TypeSignature::PrincipalType)
    ]
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_panic(
        val in strategies_for_control_flow()
        .prop_flat_map(move |ty| {
            prop_oneof![
                Just(TypeSignature::new_option(ty.clone()).unwrap()),
                Just(TypeSignature::new_response(ty.clone(), ty.clone()).unwrap()),
                Just(TypeSignature::new_response(ty.clone(), TypeSignature::NoType).unwrap()),
                Just(TypeSignature::new_response(TypeSignature::NoType, ty.clone()).unwrap())
            ]
        })
        .prop_flat_map(PropValue::from_type)
    ) {
        crosscheck(
            &format!(r#"(unwrap-panic (some {val}))"#),
            Ok(Some(val.into()))
        );
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn unwrap_err_panic(
        val in strategies_for_control_flow()
        .prop_flat_map(move |ty| {
            prop_oneof![
                Just(TypeSignature::new_response(ty.clone(), ty.clone()).unwrap()),
                Just(TypeSignature::new_response(ty.clone(), TypeSignature::NoType).unwrap()),
                Just(TypeSignature::new_response(TypeSignature::NoType, ty.clone()).unwrap())
            ]
        })
        .prop_flat_map(PropValue::from_type)
    ) {
        crosscheck(
            &format!(r#"(unwrap-err-panic (err {val}))"#),
            Ok(Some(val.into()))
        );
    }
}
