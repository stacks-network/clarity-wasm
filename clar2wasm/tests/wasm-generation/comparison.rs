use clar2wasm::tools::crosscheck_compare_only;
use clarity::vm::types::{SequenceSubtype, TypeSignature};
use clarity::vm::Value;
use proptest::strategy::{Just, Strategy};
use proptest::{prop_oneof, proptest};

use crate::{buffer, int, string_ascii, uint, PropValue};

const COMPARISONS_FUNC: [&str; 4] = ["<", "<=", ">", ">="];
const INVALID_CHAR: char = '\'';

fn filter_invalid_char(input: &Value) -> PropValue {
    let v: String = input.clone().expect_ascii().replace(INVALID_CHAR, "");

    PropValue::from(Value::from(clarity::vm::types::ASCIIData {
        data: v.into(),
    }))
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_comparison_string_ascii(val1 in string_ascii(50u32), val2 in string_ascii(50u32)) {

        let v1 = filter_invalid_char(&val1);
        let v2 = filter_invalid_char(&val2);

        for func in &COMPARISONS_FUNC {
            crosscheck_compare_only(
                &format!("({func} {v1} {v2})")
            )
        }
    }
}

fn strategies_for_comparison() -> impl Strategy<Value = TypeSignature> {
    prop_oneof![
        Just(TypeSignature::IntType),
        Just(TypeSignature::UIntType),
        (0u32..128).prop_map(|s| TypeSignature::SequenceType(SequenceSubtype::BufferType(
            s.try_into().unwrap()
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
