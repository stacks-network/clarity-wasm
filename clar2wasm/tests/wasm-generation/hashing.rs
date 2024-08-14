use clar2wasm::tools::crosscheck_validate;
use clarity::vm::types::{SequenceSubtype, TypeSignature};
use proptest::strategy::{Just, Strategy};
use proptest::{prop_oneof, proptest};

use crate::PropValue;

fn strategies_for_hashing() -> impl Strategy<Value = PropValue> {
    prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        (0u32..=300).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
        ))),
    ]
    .prop_flat_map(PropValue::from_type)
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_hashing_hash160(val in strategies_for_hashing())
    {
        crosscheck_validate(
            &format!("(hash160 {val})"), |_|{}
        )
    }

    #[test]
    fn crossprop_hashing_keccak256(val in strategies_for_hashing())
    {
        crosscheck_validate(
            &format!("(keccak256 {val})"), |_|{}
        )
    }

    #[test]
    fn crossprop_hashing_sha256(val in strategies_for_hashing())
    {
        crosscheck_validate(
            &format!("(sha256 {val})"), |_|{}
        )
    }

    #[test]
    fn crossprop_hashing_sha512(val in strategies_for_hashing())
    {
        crosscheck_validate(
            &format!("(sha512 {val})"), |_|{}
        )
    }

    #[test]
    fn crossprop_hashing_sha512_256(val in strategies_for_hashing())
    {
        crosscheck_validate(
            &format!("(sha512/256 {val})"), |_|{}
        )
    }
}
