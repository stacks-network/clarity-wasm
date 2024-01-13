/// This files purpose is to add examples of generated values that failed,
/// so that we can be sure they won't fail again after some random refactor
/// in the future.
use clar2wasm::tools::evaluate;
use clarity::vm::types::{ListData, ListTypeData, ResponseData, SequenceData, TypeSignature};
use clarity::vm::Value;
use hex::FromHex as _;

use crate::{check_against_interpreter, PropValue};

fn evaluate_expression(expr: &str) {
    let v: PropValue = evaluate(expr)
        .unwrap()
        .expect("Failed to evaluate expression")
        .into();
    assert_eq!(expr, v.to_string());
}

#[test]
fn list_ok_response() {
    evaluate_expression(
        r#"(list (ok (ok -1475)) (ok (err u115911259112154807243168097824046874107)))"#,
    )
}

#[test]
fn list_err_response() {
    evaluate_expression(
        r#"(list (err (ok -1475)) (err (err u115911259112154807243168097824046874107)))"#,
    )
}

#[test]
fn list_some_response() {
    evaluate_expression(
        r#"(list (some (ok -1475)) (some (err u115911259112154807243168097824046874107)))"#,
    )
}

#[test]
fn to_consensus_buff_1() {
    check_against_interpreter(
        "(to-consensus-buff? (err {a: 1}))",
        Some(
            Value::some(
                Value::buff_from(
                    Vec::from_hex("080c0000000101610000000000000000000000000000000001").unwrap(),
                )
                .unwrap(),
            )
            .unwrap(),
        ),
    );
}

#[test]
fn is_eq_list_opt_resp() {
    let l = "(list none (some (ok 1)))";
    check_against_interpreter(&format!(r#"(is-eq {l} {l})"#), Some(Value::Bool(true)));
}

#[test]
fn default_to() {
    check_against_interpreter(
        "(default-to (list 100) (some (list 1 2 3)))",
        Some(Value::Sequence(SequenceData::List(ListData {
            data: vec![Value::Int(1), Value::Int(2), Value::Int(3)],
            type_signature: ListTypeData::new_list(TypeSignature::IntType, 3).unwrap(),
        }))),
    );
}

#[test]
fn default_to_2() {
    check_against_interpreter(
        "(default-to (err -8865319038999812741356205373046857778) (some (ok 94740629357611018681632671610045749418)))",
        Some(Value::Response(ResponseData{
            committed: true,
            data: Box::new(Value::Int(94740629357611018681632671610045749418))
        }))
    );
}
