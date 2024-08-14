use clar2wasm::tools::crosscheck_compare_only;
use clarity::vm::types::{SequenceSubtype, TypeSignature};
use proptest::strategy::{Just, Strategy};
use proptest::{prop_oneof, proptest};

use crate::PropValue;

const HASH_FUNC: [&str; 5] = ["hash160", "keccak256", "sha256", "sha512", "sha512/256"];

fn strategies_for_hashing() -> impl Strategy<Value = TypeSignature> {
    prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        (0u32..=300).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
        ))),
    ]
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_hashing(val in strategies_for_hashing().prop_flat_map(PropValue::from_type))
    {
        for func in &HASH_FUNC {
            crosscheck_compare_only(
                &format!("({func} {val})")
            )
        }
    }
}
