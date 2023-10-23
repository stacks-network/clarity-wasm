use crate::wasm_generator::ArgumentsExt;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct GetBlockInfo;

impl Word for GetBlockInfo {
    fn name(&self) -> ClarityName {
        "get-block-info?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let prop_name = args.get_name(0)?;
        let block = args.get_expr(1)?;

        // Push the property name onto the stack
        let (id_offset, id_length) = generator.add_identifier_string_literal(prop_name);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the block number onto the stack
        generator.traverse_expr(builder, block)?;

        // Reserve space on the stack for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .expect("get-block-info? expression must be typed")
            .clone();

        let (return_offset, return_size) = generator.create_call_stack_local(
            builder,
            generator.stack_pointer,
            &return_ty,
            true,
            true,
        );

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `get_block_info`
        builder.call(generator.func_by_name("get_block_info"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty);

        Ok(())
    }
}
