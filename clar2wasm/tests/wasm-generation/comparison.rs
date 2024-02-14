use clar2wasm::tools::crosscheck_compare_only;
use clarity::vm::types::{SequenceSubtype, StringSubtype, TypeSignature};
use proptest::strategy::{Just, Strategy};
use proptest::{prop_oneof, proptest};

use crate::PropValue;

const COMPARISONS_FUNC: [&str; 4] = ["<", "<=", ">", ">="];

fn strategies_for_comparison() -> impl Strategy<Value = TypeSignature> {
    prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
        ))),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::StringType(
            StringSubtype::ASCII(s.try_into().unwrap())
        )))
    ]
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_comparison(
        (val1, val2) in strategies_for_comparison()
            .prop_flat_map(|v| (PropValue::from_type(v.clone()), PropValue::from_type(v))))
    {
        for func in &COMPARISONS_FUNC {
            crosscheck_compare_only(
                &format!("({func} {val1} {val2})")
            )
        }
    }
}
