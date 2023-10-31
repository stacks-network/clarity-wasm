use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct Let;

impl Word for Let {
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

        // Save the current locals
        let saved_locals = generator.locals.clone();

        // Traverse the bindings
        for i in 0..bindings.len() {
            let pair = bindings.get_list(i)?;
            let name = pair.get_name(0)?;
            let value = pair.get_expr(1)?;

            // Traverse the value
            generator.traverse_expr(builder, value)?;

            // Store store the value in locals, and save to the var map
            let ty = generator
                .get_expr_type(value)
                .expect("let value expression must be typed")
                .clone();
            let locals = generator.save_to_locals(builder, &ty, true);

            // Add these locals to the map
            generator.locals.insert(name.to_string(), locals);
        }

        // Traverse the body
        generator.traverse_statement_list(builder, &args[1..])?;

        // Restore the locals
        generator.locals = saved_locals;

        Ok(())
    }
}
