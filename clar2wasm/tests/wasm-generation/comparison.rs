use clar2wasm::tools::crosscheck_compare_only;
use clarity::vm::Value;
use proptest::proptest;

use crate::string_ascii;

fn filter_invalid_char(input: &Value) -> Value {
    let invalid_chars: [char; 1] = ['\''];
    let v: String = input.clone().expect_ascii().replace(invalid_chars, "");

    Value::Sequence(clarity::vm::types::SequenceData::String(
        clarity::vm::types::CharType::ASCII(clarity::vm::types::ASCIIData { data: v.into() }),
    ))
}

const COMPARISONS_FUNC: [&str; 4] = ["<", "<=", ">", ">="];

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
