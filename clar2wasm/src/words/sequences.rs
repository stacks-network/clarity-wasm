use clarity::vm::{
    clarity_wasm::get_type_size,
    types::{SequenceSubtype, StringSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};
use walrus::{
    ir::{BinaryOp, UnaryOp},
    ValType,
};

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

#[derive(Debug)]
pub struct AsMaxLen;

impl Word for AsMaxLen {
    fn name(&self) -> ClarityName {
        "as-max-len?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 2 {
            return Err(GeneratorError::InternalError(
                "expected two arguments to 'as-max-len?'".to_string(),
            ));
        }

        // Push a `0` and a `1` to the stack, to be used by the `select`
        // instruction later.
        builder.i32_const(0).i32_const(1);

        // Traverse the input list, leaving the offset and length on top of
        // the stack.
        generator.traverse_expr(builder, &args[0])?;

        // Save the offset and length to locals for later. Leave the length on
        // top of the stack.
        let length_local = generator.module.locals.add(ValType::I32);
        builder.local_set(length_local);
        let offset_local = generator.module.locals.add(ValType::I32);
        builder.local_set(offset_local);
        builder.local_get(length_local);

        // We need to check if the list is longer than the second argument.
        // If it is, then return `none`, otherwise, return `(some input)`.
        // Push the length of the value onto the stack.

        // Get the length.
        generator
            .get_expr_type(&args[0])
            .ok_or_else(|| GeneratorError::InternalError("append result must be typed".to_string()))
            .and_then(|ty| match ty {
                TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                    // The length of the list in bytes is on the top of the stack. If we
                    // divide that by the length of each element, then we'll have the
                    // length of the list in elements.
                    let element_length = get_type_size(list.get_list_item_type());
                    builder.i32_const(element_length);

                    // Divide the length of the list by the length of each element to get
                    // the number of elements in the list.
                    builder.binop(BinaryOp::I32DivU);

                    Ok(())
                }
                TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    _,
                ))) => Err(GeneratorError::NotImplemented),
                // The byte length of buffers and ASCII strings is the same as
                // the value length, so just leave it as-is.
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => Ok(()),
                _ => Err(GeneratorError::InternalError(
                    "expected sequence type".to_string(),
                )),
            })?;

        // Convert this 32-bit length to a 64-bit value, for comparison.
        builder.unop(UnaryOp::I64ExtendUI32);

        // Traverse the second argument, the desired length, leaving the low
        // and high parts on the stack, then drop the high part.
        generator.traverse_expr(builder, &args[1])?;
        builder.drop();

        // Compare the length of the list to the desired length.
        builder.binop(BinaryOp::I64GeU);

        // Select from the `0` and `1` that we pushed to the stack earlier,
        // based on the result of the comparison.
        builder.select(Some(ValType::I32));

        // Now, put the original offset and length back on the stack. In the
        // case where the result is `none`, these will be ignored, but it
        // doesn't hurt to have them there.
        builder.local_get(offset_local).local_get(length_local);

        Ok(())
    }
}

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
        if args.len() != 2 {
            return Err(GeneratorError::InternalError(
                "expected two arguments to 'as-max-len?'".to_string(),
            ));
        }

        let memory = generator.get_memory();

        // Create a new sequence to hold the result in the stack frame
        let ty = generator
            .get_expr_type(expr)
            .expect("concat expression must be typed")
            .clone();
        let (offset, _) = generator.create_call_stack_local(builder, &ty, false, true);
        builder.local_get(offset);

        // Traverse the lhs, leaving it on the data stack (offset, size)
        generator.traverse_expr(builder, &args[0])?;

        // Save the length of the lhs
        let lhs_length = generator.module.locals.add(ValType::I32);
        builder.local_tee(lhs_length);

        // Copy the lhs to the new sequence
        builder.memory_copy(memory, memory);

        // Load the adjusted destination offset
        builder
            .local_get(offset)
            .local_get(lhs_length)
            .binop(BinaryOp::I32Add);

        // Traverse the rhs, leaving it on the data stack (offset, size)
        generator.traverse_expr(builder, &args[1])?;

        // Save the length of the rhs
        let rhs_length = generator.module.locals.add(ValType::I32);
        builder.local_tee(rhs_length);

        // Copy the rhs to the new sequence
        builder.memory_copy(memory, memory);

        // Load the offset of the new sequence
        builder.local_get(offset);

        // Total size = lhs_length + rhs_length
        builder
            .local_get(lhs_length)
            .local_get(rhs_length)
            .binop(BinaryOp::I32Add);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Len;

impl Word for Len {
    fn name(&self) -> ClarityName {
        "len".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 1 {
            return Err(GeneratorError::InternalError(
                "expected one argument to 'len'".to_string(),
            ));
        }

        // Traverse the list, leaving the offset and length on top of the stack.
        generator.traverse_expr(builder, &args[0])?;

        // Save the length, then drop the offset and push the length back.
        let length_local = generator.module.locals.add(ValType::I32);
        builder
            .local_set(length_local)
            .drop()
            .local_get(length_local);

        // Get the length
        generator
            .get_expr_type(&args[0])
            .ok_or_else(|| GeneratorError::InternalError("append result must be typed".to_string()))
            .and_then(|ty| match ty {
                TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                    // The length of the list in bytes is on the top of the stack. If we
                    // divide that by the length of each element, then we'll have the
                    // length of the list in elements.
                    let element_length = get_type_size(list.get_list_item_type());
                    builder.i32_const(element_length);

                    // Divide the length of the list by the length of each element to get
                    // the number of elements in the list.
                    builder.binop(BinaryOp::I32DivU);

                    Ok(())
                }
                TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    _,
                ))) => Err(GeneratorError::NotImplemented),
                // The byte length of buffers and ASCII strings is the same as
                // the value length, so just leave it as-is.
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => Ok(()),
                _ => Err(GeneratorError::InternalError(
                    "expected sequence type".to_string(),
                )),
            })?;

        // Convert this 32-bit length to a 64-bit value.
        builder.unop(UnaryOp::I64ExtendUI32);

        // Then push a 0 for the upper 64 bits.
        builder.i64_const(0);

        Ok(())
    }
}
