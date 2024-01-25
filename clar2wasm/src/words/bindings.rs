use clarity::vm::{ClarityName, SymbolicExpression};

use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use crate::words::ComplexWord;

#[derive(Debug)]
pub struct Let;

impl ComplexWord for Let {
    fn name(&self) -> ClarityName {
        "let".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let bindings = args.get_list(0)?;

        // Save the current named locals
        let saved_locals = generator.bindings.clone();

        // Traverse the bindings
        for i in 0..bindings.len() {
            let pair = bindings.get_list(i)?;
            let name = pair.get_name(0)?;
            let value = pair.get_expr(1)?;

            // make sure name does not collide with builtin or user-defined symbols

            if generator.is_reserved_name(name) {
                return Err(GeneratorError::InternalError(format!(
                    "Name already used {:?}",
                    name
                )));
            }

            // Traverse the value
            generator.traverse_expr(builder, value)?;

            // Store store the value in locals, and save to the var map
            let ty = generator
                .get_expr_type(value)
                .ok_or_else(|| {
                    GeneratorError::TypeError("let value expression must be typed".to_owned())
                })?
                .clone();
            let locals = generator.save_to_locals(builder, &ty, true);

            // Add these named locals to the map
            generator.bindings.insert(name.to_string(), locals);
        }

        // WORKAROUND: need to set the last statement type to the type of the let expression
        let expr_ty = generator
            .get_expr_type(_expr)
            .ok_or_else(|| GeneratorError::TypeError("let expression should be typed".to_owned()))?
            .clone();
        generator.set_expr_type(
            args.last().ok_or_else(|| {
                GeneratorError::TypeError(
                    "let expression should have at least one statement".to_owned(),
                )
            })?,
            expr_ty,
        );

        // Traverse the body
        generator.traverse_statement_list(builder, &args[1..])?;

        // Restore the named locals
        generator.bindings = saved_locals;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::{crosscheck, crosscheck_compare_only};

    #[test]
    fn clar_let_disallow_builtin_names() {
        // It's not allowed to use names of user-defined functions as bindings
        const ERR: &str = "
(define-private (test)
 (let ((+ u3))
   +))";

        crosscheck(&format!("{ERR} (test)"), Err(()));
    }

    #[test]
    fn clar_let_disallow_user_defined_names() {
        // It's not allowed to use names of user-defined functions as bindings
        const ERR: &str = "
(define-private (test)
 (let ((test u3))
    test))";

        crosscheck(&format!("{ERR} (test)"), Err(()));
    }

    #[test]
    fn let_with_multiple_statements() {
        crosscheck_compare_only(
            r#"
                (define-data-var count uint u0)

                (define-public (decrement)
                    (let ((current-count (var-get count)))
                        (asserts! (> current-count u0) (err u1))
                        (ok (var-set count (- current-count u1)))
                    )
                )
            "#,
        )
    }
}
