use clarity::vm::clarity_wasm::get_type_in_memory_size;
use clarity::vm::{ClarityName, SymbolicExpression, SymbolicExpressionType};
use walrus::ValType;

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct DefineConstant;

impl ComplexWord for DefineConstant {
    fn name(&self) -> ClarityName {
        "define-constant".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let value = args.get_expr(1)?;

        // If the initial value is a literal, then we can directly add it to
        // the literal memory.
        let offset = if let SymbolicExpressionType::LiteralValue(value) = &value.expr {
            let (offset, _len) = generator.add_literal(value)?;
            offset
        } else {
            // Traverse the initial value expression.
            generator.traverse_expr(builder, value)?;

            // If the initial expression is not a literal, then we need to
            // reserve the space for it, and then execute the expression and
            // write the result into the reserved space.
            let offset = generator.literal_memory_end;
            let offset_local = generator.module.locals.add(ValType::I32);
            builder.i32_const(offset as i32).local_set(offset_local);

            let ty = generator
                .get_expr_type(value)
                .ok_or_else(|| {
                    GeneratorError::TypeError("constant value must be typed".to_owned())
                })?
                .clone();

            let len = get_type_in_memory_size(&ty, true) as u32;
            generator.literal_memory_end += len;

            // Write the initial value to the memory, to be read by the host.
            generator.write_to_memory(builder, offset_local, 0, &ty)?;

            offset
        };

        generator.constants.insert(name.to_string(), offset);

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::unimplemented)]
mod tests {
    use clarity::vm::types::{ListData, ListTypeData, SequenceData};
    use clarity::vm::Value;

    use crate::tools::crosscheck;

    #[test]
    fn define_constant_const() {
        crosscheck(
            r#"(define-constant four 4) (define-private (go) (print four)) (go)"#,
            Ok(Some(Value::Int(4))),
        )
    }

    #[test]
    fn define_constant_function() {
        crosscheck(
            r#"(define-constant four (+ 2 2)) (define-private (go) (print four)) (go)"#,
            Ok(Some(Value::Int(4))),
        )
    }

    #[test]
    fn define_constant_list() {
        crosscheck(
            r#"(define-constant list-of-2-int (list 1 1)) (define-private (go) (print list-of-2-int)) (go)"#,
            Ok(Some(Value::Sequence(SequenceData::List(ListData {
                data: vec![Value::Int(1), Value::Int(1)],
                type_signature: ListTypeData::new_list(
                    clarity::vm::types::TypeSignature::IntType,
                    2,
                )
                .unwrap(),
            })))),
        )
    }
}
