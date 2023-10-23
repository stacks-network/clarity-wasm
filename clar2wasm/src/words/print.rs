use crate::wasm_generator::{clar2wasm_ty, ArgumentsExt};
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ValType;

use super::Word;

#[derive(Debug)]
pub struct Print;

impl Word for Print {
    fn name(&self) -> ClarityName {
        "print".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let value = args.get_expr(0)?;

        // Traverse the value, leaving it on the data stack
        generator.traverse_expr(builder, value)?;

        // Save the value to locals
        let wasm_types = clar2wasm_ty(
            generator
                .get_expr_type(value)
                .expect("print value expression must be typed"),
        );
        let mut val_locals = Vec::with_capacity(wasm_types.len());
        for local_ty in wasm_types.iter().rev() {
            let local = generator.module.locals.add(*local_ty);
            val_locals.push(local);
            builder.local_set(local);
        }
        val_locals.reverse();

        // Save the offset (current stack pointer) into a local.
        // This is where we will serialize the value to.
        let offset = generator.module.locals.add(ValType::I32);
        let length = generator.module.locals.add(ValType::I32);
        builder
            .global_get(generator.stack_pointer)
            .local_set(offset);

        let ty = generator
            .get_expr_type(value)
            .expect("print value expression must be typed")
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
        builder.call(generator.func_by_name("print"));

        // Print always returns its input, so read the input value back from
        // the locals.
        for val_local in val_locals {
            builder.local_get(val_local);
        }

        Ok(())
    }
}
