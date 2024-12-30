use clarity::vm::types::{ListTypeData, PrincipalData, SequenceSubtype, TypeSignature};
use clarity::vm::Value;

use crate::{crosscheck_oom, crosscheck_oom_with_non_literal_args, list_of};

#[test]
#[ignore = "issue #585"]
fn principal_of_oom() {
    crosscheck_oom(
        "(principal-of? 0x03adb8de4bfb65db2cfd6120d55c6526ae9c52e675db7e47308636534ba7786110)",
        Ok(Some(
            Value::okay(
                PrincipalData::parse("ST1AW6EKPGT61SQ9FNVDS17RKNWT8ZP582VF9HSCP")
                    .unwrap()
                    .into(),
            )
            .unwrap(),
        )),
    )
}

#[test]
fn list_oom() {
    crosscheck_oom(
        "(list 1 2 3)",
        Ok(Some(
            Value::cons_list_unsanitized(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
                .unwrap(),
        )),
    );
}

#[test]
fn append_oom() {
    crosscheck_oom_with_non_literal_args(
        "(append (list 1 2 3) 4)",
        &[list_of(TypeSignature::IntType, 3)],
        Ok(Some(
            Value::cons_list_unsanitized(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
            ])
            .unwrap(),
        )),
    );
}

#[test]
fn concat_oom() {
    crosscheck_oom_with_non_literal_args(
        "(concat (list 1 2 3) (list 4 5))",
        &[
            list_of(TypeSignature::IntType, 3),
            list_of(TypeSignature::IntType, 2),
        ],
        Ok(Some(
            Value::cons_list_unsanitized(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ])
            .unwrap(),
        )),
    );
}

#[test]
fn replace_at_oom() {
    crosscheck_oom_with_non_literal_args(
        "(replace-at? (list 1 2 3) u0 42)",
        &[list_of(TypeSignature::IntType, 3)],
        Ok(Some(
            Value::some(
                Value::cons_list_unsanitized(vec![Value::Int(42), Value::Int(2), Value::Int(3)])
                    .unwrap(),
            )
            .unwrap(),
        )),
    );
}

#[test]
fn map_oom() {
    crosscheck_oom_with_non_literal_args(
        "(define-private (foo (b bool)) (if b u1 u0)) (map foo (list true true false))",
        &[list_of(TypeSignature::BoolType, 3)],
        Ok(Some(
            Value::cons_list_unsanitized(vec![Value::UInt(1), Value::UInt(1), Value::UInt(0)])
                .unwrap(),
        )),
    )
}

#[test]
fn fold_oom() {
    let snippet = r#"
(define-private (concat-buff (a (buff 1)) (b (buff 3)))
  (unwrap-panic (as-max-len? (concat a b) u3)))
(fold concat-buff 0x010203 0x)
    "#;
    crosscheck_oom(snippet, Ok(Some(Value::buff_from(vec![3, 2, 1]).unwrap())));
}
