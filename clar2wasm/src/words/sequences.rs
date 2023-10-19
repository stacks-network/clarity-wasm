use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::{ir::BinaryOp, ValType};

use crate::wasm_generator::GeneratorError;

use super::Word;

#[derive(Debug)]
pub struct Append;

impl Word for Append {
    fn name(&self) -> ClarityName {
        "append".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 2 {
            return Err(GeneratorError::InternalError(
                "expected two arguments to 'append'".to_string(),
            ));
        }

        let ty = generator
            .get_expr_type(expr)
            .ok_or(GeneratorError::InternalError(
                "append result must be typed".to_string(),
            ))?
            .clone();

        let memory = generator.get_memory();

        // Allocate stack space for the new list.
        let (write_ptr, length) = generator.create_call_stack_local(builder, &ty, false, true);

        // Push the offset and length of this list to the stack to be returned.
        builder.local_get(write_ptr).i32_const(length);

        // Push the write pointer onto the stack for `memory.copy`.
        builder.local_get(write_ptr);

        // Traverse the list to append to, leaving the offset and length on
        // top of the stack.
        generator.traverse_expr(builder, &args[0])?;

        // The stack now has the destination, source and length arguments in
        // right order for `memory.copy` to copy the source list into the new
        // list. Save a copy of the length for later.
        let src_length = generator.module.locals.add(ValType::I32);
        builder.local_tee(src_length);
        builder.memory_copy(memory, memory);

        // Increment the write pointer by the length of the source list.
        builder
            .local_get(write_ptr)
            .local_get(src_length)
            .binop(BinaryOp::I32Add)
            .local_set(write_ptr);

        // Traverse the element that we're appending to the list.
        generator.traverse_expr(builder, &args[1])?;

        // Get the type of the element that we're appending.
        let elem_ty = generator
            .get_expr_type(&args[1])
            .ok_or(GeneratorError::InternalError(
                "append element must be typed".to_string(),
            ))?
            .clone();

        // Store the element at the write pointer.
        generator.write_to_memory(builder, write_ptr, 0, &elem_ty);

        Ok(())
    }
}
