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
mod tests {
    use clarity::vm::types::{ListData, ListTypeData, SequenceData};
    use clarity::vm::Value;

    use crate::tools::{crosscheck, evaluate};

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

    #[test]
    fn test_int_constant() {
        crosscheck(
            "
(define-constant small-int 1)
(define-public (get-int-constant)
  (ok small-int))
(get-int-constant)",
            evaluate("(ok 1)"),
        );
    }

    #[test]
    fn test_large_uint_constant() {
        crosscheck(
            "
(define-constant large-uint u338770000845734292516042252062085074415)
(define-public (get-large-uint-constant)
  (ok large-uint))
(get-large-uint-constant)",
            evaluate("(ok u338770000845734292516042252062085074415)"),
        );
    }

    #[test]
    fn test_string_constant() {
        crosscheck(
            r#"
(define-constant string "hello world")
(define-public (get-string-constant)
  (ok string))
(get-string-constant)"#,
            evaluate(r#"(ok "hello world")"#),
        );
    }

    #[test]
    fn test_string_utf8_constant() {
        crosscheck(
            r#"
(define-constant string-utf8 u"hello world\u{1F98A}")
(define-public (get-string-utf8-constant)
  (ok string-utf8))
(get-string-utf8-constant)
"#,
            evaluate(r#"(ok u"hello world\u{1F98A}")"#),
        );
    }

    #[test]
    fn test_bytes_constant() {
        crosscheck(
            "
(define-constant bytes 0x12345678)
(define-public (get-bytes-constant)
  (ok bytes))
(get-bytes-constant)
",
            evaluate("(ok 0x12345678)"),
        );
    }
}
