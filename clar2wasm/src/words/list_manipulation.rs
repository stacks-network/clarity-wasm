use crate::wasm_generator::{ArgumentsExt, GeneratorError};
use clarity::vm::clarity_wasm::get_type_size;
use clarity::vm::{
    types::{SequenceSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};
use walrus::ir::{BinaryOp, InstrSeqType};
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

#[derive(Debug)]
pub struct Fold;

impl Word for Fold {
    fn name(&self) -> ClarityName {
        "fold".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let func = args.get_name(0)?;
        let sequence = args.get_expr(1)?;
        let initial = args.get_expr(2)?;

        // Fold takes an initial value, and a sequence, and applies a function
        // to the output of the previous call, or the initial value in the case
        // of the first call, and each element of the sequence.
        // ```
        // (fold - (list 2 4 6) 0)
        // ```
        // is equivalent to
        // ```
        // (- 6 (- 4 (- 2 0)))
        // ```

        // The result type must match the type of the initial value
        let result_clar_ty = generator
            .get_expr_type(initial)
            .expect("fold's initial value expression must be typed");
        let result_ty = crate::wasm_generator::clar2wasm_ty(result_clar_ty);
        let loop_body_ty = InstrSeqType::new(&mut generator.module.types, &[], &[]);

        // Get the type of the sequence
        let seq_ty = match generator
            .get_expr_type(sequence)
            .expect("sequence expression must be typed")
        {
            TypeSignature::SequenceType(seq_ty) => seq_ty.clone(),
            _ => {
                return Err(GeneratorError::InternalError(
                    "expected sequence type".to_string(),
                ));
            }
        };

        let (seq_len, elem_ty) = match &seq_ty {
            SequenceSubtype::ListType(list_type) => {
                (list_type.get_max_len(), list_type.get_list_item_type())
            }
            _ => unimplemented!("Unsupported sequence type"),
        };

        // Evaluate the sequence, which will load it into the call stack,
        // leaving the offset and size on the data stack.
        generator.traverse_expr(builder, sequence)?;

        // Drop the size, since we don't need it
        builder.drop();

        // Store the offset into a local
        let offset = generator.module.locals.add(ValType::I32);
        builder.local_set(offset);

        let elem_size = get_type_size(elem_ty);

        // Store the end of the sequence into a local
        let end_offset = generator.module.locals.add(ValType::I32);
        builder
            .local_get(offset)
            .i32_const(seq_len as i32 * elem_size)
            .binop(BinaryOp::I32Add)
            .local_set(end_offset);

        // Evaluate the initial value, so that its result is on the data stack
        generator.traverse_expr(builder, initial)?;

        if seq_len == 0 {
            // If the sequence is empty, just return the initial value
            return Ok(());
        }

        // Define local(s) to hold the intermediate result, and initialize them
        // with the initial value. Not that we are looping in reverse order, to
        // pop values from the top of the stack.
        let mut result_locals = Vec::with_capacity(result_ty.len());
        for local_ty in result_ty.iter().rev() {
            let local = generator.module.locals.add(*local_ty);
            result_locals.push(local);
            builder.local_set(local);
        }
        result_locals.reverse();

        // Define the body of a loop, to loop over the sequence and make the
        // function call.
        builder.loop_(loop_body_ty, |loop_| {
            let loop_id = loop_.id();

            // Load the element from the sequence
            let elem_size = generator.read_from_memory(loop_, offset, 0, elem_ty);

            // Push the locals to the stack
            for result_local in result_locals.iter() {
                loop_.local_get(*result_local);
            }

            // Call the function
            loop_.call(generator.func_by_name(func.as_str()));

            // Save the result into the locals (in reverse order as we pop)
            for result_local in result_locals.iter().rev() {
                loop_.local_set(*result_local);
            }

            // Increment the offset by the size of the element, leaving the
            // offset on the top of the stack
            loop_
                .local_get(offset)
                .i32_const(elem_size)
                .binop(BinaryOp::I32Add)
                .local_tee(offset);

            // Loop if we haven't reached the end of the sequence
            loop_
                .local_get(end_offset)
                .binop(BinaryOp::I32LtU)
                .br_if(loop_id);
        });

        // Push the locals to the stack
        for result_local in result_locals.iter() {
            builder.local_get(*result_local);
        }

        Ok(())
    }
}
