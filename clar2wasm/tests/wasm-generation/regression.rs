/// This files purpose is to add examples of generated values that failed,
/// so that we can be sure they won't fail again after some random refactor
/// in the future.
use clar2wasm::tools::evaluate;
use clarity::vm::Value;
use hex::FromHex as _;

use crate::PropValue;

fn evaluate_expression(expr: &str) {
    let v: PropValue = evaluate(expr)
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
    assert_eq!(
        evaluate(r#"(to-consensus-buff? (err {a: 1}))"#,),
        Some(
            Value::some(
                Value::buff_from(
                    Vec::from_hex("080c0000000101610000000000000000000000000000000001").unwrap()
                )
                .unwrap()
            )
            .unwrap()
        )
    );
}

#[test]
fn is_eq_list_opt_resp() {
    let l = "(list none (some (ok 1)))";
    assert_eq!(
        evaluate(&format!(r#"(is-eq {l} {l})"#)),
        Some(Value::Bool(true))
    )
}
