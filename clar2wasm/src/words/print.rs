use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ValType;

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct Print;

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
            .ok_or(GeneratorError::TypeError(
                "print value expression must be typed".to_owned(),
            ))?
            .clone();
        let val_locals = generator.save_to_locals(builder, &ty, true);

        // Save the offset (current stack pointer) into a local.
        // This is where we will serialize the value to.
        let offset = generator.module.locals.add(ValType::I32);
        let length = generator.module.locals.add(ValType::I32);
        builder
            .global_get(generator.stack_pointer)
            .local_set(offset);

        let ty = generator
            .get_expr_type(value)
            .ok_or(GeneratorError::TypeError(
                "print value expression must be typed".to_owned(),
            ))?
            .clone();

        // Push the value back onto the data stack
        for val_local in &val_locals {
            builder.local_get(*val_local);
        }

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
