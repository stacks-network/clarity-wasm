use clarity::vm::types::signatures::CallableSubtype;
use clarity::vm::types::{
    ASCIIData, CharType, ListTypeData, SequenceSubtype, TupleTypeSignature, TypeSignature,
};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use crate::wasm_utils::signature_from_string;

#[derive(Debug)]
pub struct Print;

/// Try to change `ty` for serialization/deserialization (as stringified signature)
/// In case of failure, clones the input `ty`
fn type_for_serialization(ty: &TypeSignature) -> TypeSignature {
    use clarity::vm::types::signatures::TypeSignature::*;
    match ty {
        // NoType and BoolType have the same size (both type and inner)
        NoType => BoolType,
        // Avoid serialization like `(list 2 <S1G2081040G2081040G2081040G208105NK8PE5.my-trait.my-trait>)`
        CallableType(CallableSubtype::Trait(_)) => PrincipalType,
        // Recursive types
        ResponseType(types) => ResponseType(Box::new((
            type_for_serialization(&types.0),
            type_for_serialization(&types.1),
        ))),
        OptionalType(value_ty) => OptionalType(Box::new(type_for_serialization(value_ty))),
        SequenceType(SequenceSubtype::ListType(list_ty)) => {
            SequenceType(SequenceSubtype::ListType(
                ListTypeData::new_list(
                    type_for_serialization(list_ty.get_list_item_type()),
                    list_ty.get_max_len(),
                )
                .unwrap_or_else(|_| list_ty.clone()),
            ))
        }
        TupleType(tuple_ty) => TupleType(
            TupleTypeSignature::try_from(
                tuple_ty
                    .get_type_map()
                    .iter()
                    .map(|(k, v)| (k.clone(), type_for_serialization(v)))
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| tuple_ty.clone()),
        ),
        t => t.clone(),
    }
}

impl ComplexWord for Print {
    fn name(&self) -> ClarityName {
        "print".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let value = args.get_expr(0)?;

        // Traverse the value, leaving it on the data stack
        generator.traverse_expr(builder, value)?;

        // Save the value to locals
        let ty = generator
            .get_expr_type(value)
            .ok_or_else(|| {
                GeneratorError::TypeError("print value expression must be typed".to_owned())
            })?
            .clone();
        let val_locals = generator.save_to_locals(builder, &ty, true);

        let ty_for_serde = type_for_serialization(&ty);
        let serialized_ty = ty_for_serde.to_string();
        // Ensure (at compile time) type can be reconstructed
        signature_from_string(
            &serialized_ty,
            generator.contract_analysis.clarity_version,
            generator.contract_analysis.epoch,
        )
        .map_err(|e| {
            GeneratorError::TypeError(format!("serialized type cannot be deserialized: {e:?}"))
        })?;
        let serialized_ty = serialized_ty.bytes().collect();

        let (serialized_ty_offset, serialized_ty_len) =
            generator.add_clarity_string_literal(&CharType::ASCII(ASCIIData {
                data: serialized_ty,
            }))?;

        // Push the value back onto the data stack
        for val_local in &val_locals {
            builder.local_get(*val_local);
        }
        // Storing expr to memory to pass a reference to `print`
        let (value_offset, value_length) =
            generator.create_call_stack_local(builder, &ty, false, true);
        generator.write_to_memory(builder, value_offset, 0, &ty)?;
        // Then load the offset and length onto the stack
        builder.local_get(value_offset).i32_const(value_length);

        // Push type offset and length onto the stack
        builder
            .i32_const(serialized_ty_offset as i32)
            .i32_const(serialized_ty_len as i32);

        // Call the host interface function, `print`
        builder.call(generator.func_by_name("stdlib.print"));

        // Print always returns its input, so read the input value back from
        // the locals.
        for val_local in val_locals {
            builder.local_get(val_local);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::types::StacksEpochId;
    use clarity::vm::types::{ListTypeData, TupleData};
    use clarity::vm::Value;

    use crate::tools::crosscheck_with_events;

    #[test]
    fn test_simple() {
        crosscheck_with_events("(print 42)", Ok(Some(Value::Int(42))));
    }

    #[test]
    fn test_contract_call() {
        let first_contract_name = "callee".into();
        let first_snippet = r#"
(define-public (foo (a int))
  (ok (print a))
)
            "#;

        let second_contract_name = "caller".into();
        let second_snippet = "(unwrap-panic (contract-call? .callee foo 42))";

        crate::tools::crosscheck_multi_contract(
            &[
                (first_contract_name, first_snippet),
                (second_contract_name, second_snippet),
            ],
            Ok(Some(Value::Int(42))),
        );
    }

    #[test]
    fn test_empty_list() {
        crosscheck_with_events(
            "(print (list))",
            Ok(Some(
                Value::list_with_type(
                    &StacksEpochId::latest(),
                    vec![],
                    ListTypeData::new_list(clarity::vm::types::TypeSignature::NoType, 0).unwrap(),
                )
                .unwrap(),
            )),
        );
    }

    #[test]
    fn test_complex_notype() {
        let notype_list = Value::list_with_type(
            &StacksEpochId::latest(),
            vec![],
            ListTypeData::new_list(clarity::vm::types::TypeSignature::NoType, 0).unwrap(),
        )
        .unwrap();
        let none_list = Value::cons_list(vec![Value::none()], &StacksEpochId::latest()).unwrap();
        let err = Value::err_uint(1);
        crosscheck_with_events(
            "(print { a: (list), b: (list none), c: (err u1) })",
            Ok(Some(Value::Tuple(
                TupleData::from_data(vec![
                    ("a".into(), notype_list),
                    ("b".into(), none_list),
                    ("c".into(), err),
                ])
                .unwrap(),
            ))),
        );
    }

    #[test]
    fn test_large_buff() {
        let msg = "a".repeat(1 << 20);
        crosscheck_with_events(
            &format!(r#"(print "{msg}")"#),
            Ok(Some(
                Value::string_ascii_from_bytes(msg.into_bytes()).unwrap(),
            )),
        );
    }

    #[test]
    fn test_large_serialization() {
        // `(list 162141 (string-ascii 0))` results in >1MB serialization (1_310_710)
        let n = 262141;
        crosscheck_with_events(
            &format!(
                r#"
(define-private (foo (a (string-ascii 1))) "")
(print (map foo "{}"))
"#,
                "a".repeat(n)
            ),
            Ok(Some(
                Value::cons_list(
                    (0..n)
                        .map(|_| Value::string_ascii_from_bytes(vec![]).unwrap())
                        .collect(),
                    &StacksEpochId::latest(),
                )
                .unwrap(),
            )),
        );
    }
}
