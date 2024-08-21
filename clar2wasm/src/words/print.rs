use clarity::vm::types::{ListTypeData, SequenceSubtype, TupleTypeSignature, TypeSignature};
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ValType;

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct Print;

/// Replace `NoType`s in `ty` with a `IntType` proxy
fn ignore_notype(ty: &TypeSignature) -> TypeSignature {
    use clarity::vm::types::signatures::TypeSignature::*;
    match ty {
        ResponseType(types) => {
            ResponseType(Box::new((ignore_notype(&types.0), ignore_notype(&types.1))))
        }
        OptionalType(value_ty) => OptionalType(Box::new(ignore_notype(value_ty))),
        SequenceType(SequenceSubtype::ListType(list_ty)) => {
            SequenceType(SequenceSubtype::ListType(
                ListTypeData::new_list(
                    ignore_notype(list_ty.get_list_item_type()),
                    list_ty.get_max_len(),
                )
                .unwrap(),
            ))
        }
        TupleType(tuple_ty) => TupleType(
            TupleTypeSignature::try_from(
                tuple_ty
                    .get_type_map()
                    .iter()
                    .map(|(k, v)| (k.clone(), ignore_notype(v)))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        ),
        NoType => IntType,
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

        // Save the offset (current stack pointer) into a local.
        // This is where we will serialize the value to.
        let offset = generator.module.locals.add(ValType::I32);
        let length = generator.module.locals.add(ValType::I32);
        builder
            .global_get(generator.stack_pointer)
            .local_set(offset);

        // Push the value back onto the data stack
        for val_local in &val_locals {
            builder.local_get(*val_local);
        }

        generator.ensure_work_space(ignore_notype(&ty).max_serialized_size().map_err(|_| {
            GeneratorError::TypeError(
                "cannot determine serialized expression max size to print value".to_owned(),
            )
        })?);

        // Write the serialized value to the top of the call stack
        generator.serialize_to_memory(builder, offset, 0, &ty)?;

        // Save the length to a local
        builder.local_set(length);

        // Push the offset and size to the data stack
        builder.local_get(offset).local_get(length);

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
    use clarity::vm::Value;

    use crate::tools::{crosscheck, crosscheck_compare_only};

    #[test]
    fn test_empty_list() {
        crosscheck_compare_only("(print (list))");
    }

    #[test]
    fn test_complex_notype() {
        crosscheck_compare_only("(print { a: (list), b: (list none), c: (err 1) })");
    }

    #[test]
    fn test_large_buff() {
        let msg = "a".repeat(1 << 20);
        crosscheck(
            &format!(r#"(print "{msg}")"#),
            Ok(Some(
                Value::string_ascii_from_bytes(msg.into_bytes()).unwrap(),
            )),
        );
    }
}
