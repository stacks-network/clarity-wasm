use crate::wasm_generator::{ArgumentsExt, GeneratorError};
use clarity::vm::{
    types::{SequenceSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};
use walrus::ir::BinaryOp;
use walrus::ValType;

use super::Word;

#[derive(Debug)]
pub struct Concat;

impl Word for Concat {
    fn name(&self) -> ClarityName {
        "concat".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let lhs = args.get_expr(0)?;
        let rhs = args.get_expr(1)?;

        // Create a new sequence to hold the result in the stack frame
        let ty = generator
            .get_expr_type(expr)
            .expect("concat expression must be typed")
            .clone();
        let (offset, _) =
            generator.create_call_stack_local(builder, generator.stack_pointer, &ty, false, true);

        // Traverse the lhs, leaving it on the data stack (offset, size)
        generator.traverse_expr(builder, lhs)?;

        // Retrieve the memcpy function:
        // memcpy(src_offset, length, dst_offset)
        let memcpy = generator
            .module
            .funcs
            .by_name("memcpy")
            .expect("function not found: memcpy");

        // Copy the lhs to the new sequence
        builder.local_get(offset).call(memcpy);

        // Save the new destination offset
        let end_offset = generator.module.locals.add(ValType::I32);
        builder.local_set(end_offset);

        // Traverse the rhs, leaving it on the data stack (offset, size)
        generator.traverse_expr(builder, rhs)?;

        // Copy the rhs to the new sequence
        builder.local_get(end_offset).call(memcpy);

        // Total size = end_offset - offset
        let size = generator.module.locals.add(ValType::I32);
        builder
            .local_get(offset)
            .binop(BinaryOp::I32Sub)
            .local_set(size);

        // Return the new sequence (offset, size)
        builder.local_get(offset).local_get(size);

        Ok(())
    }
}

#[derive(Debug)]
pub struct ListCons;

impl Word for ListCons {
    fn name(&self) -> ClarityName {
        "list".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        list: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let ty = generator
            .get_expr_type(expr)
            .expect("list expression must be typed")
            .clone();
        let (elem_ty, num_elem) =
            if let TypeSignature::SequenceType(SequenceSubtype::ListType(list_type)) = &ty {
                (list_type.get_list_item_type(), list_type.get_max_len())
            } else {
                panic!(
                    "Expected list type for list expression, but found: {:?}",
                    ty
                );
            };

        assert_eq!(num_elem as usize, list.len(), "list size mismatch");

        // Allocate space on the data stack for the entire list
        let (offset, size) =
            generator.create_call_stack_local(builder, generator.stack_pointer, &ty, false, true);

        // Loop through the expressions in the list and store them onto the
        // data stack.
        let mut total_size = 0;
        for expr in list.iter() {
            generator.traverse_expr(builder, expr)?;
            // Write this element to memory
            let elem_size = generator.write_to_memory(builder, offset, total_size, elem_ty);
            total_size += elem_size;
        }
        assert_eq!(total_size, size as u32, "list size mismatch");

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(size);

        Ok(())
    }
}
