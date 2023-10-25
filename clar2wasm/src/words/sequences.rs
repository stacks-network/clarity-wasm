use clarity::vm::{
    clarity_wasm::get_type_size,
    types::{SequenceSubtype, StringSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};
use walrus::{
    ir::{BinaryOp, InstrSeqType, UnaryOp},
    ValType,
};

use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError,
    WasmGenerator,
};

use super::Word;

#[derive(Debug)]
pub struct ListCons;

impl Word for ListCons {
    fn name(&self) -> ClarityName {
        "list".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
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
        let (offset, size) = generator.create_call_stack_local(builder, &ty, false, true);

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
        generator: &mut WasmGenerator,
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

#[derive(Debug)]
pub enum ElementAt {
    Original,
    Alias,
}

impl Word for ElementAt {
    fn name(&self) -> ClarityName {
        match self {
            ElementAt::Original => "element-at".into(),
            ElementAt::Alias => "element-at?".into(),
        }
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
                "expected two arguments to 'element-at?'".to_string(),
            ));
        }

        // Traverse the list, leaving the offset and length on top of the stack.
        generator.traverse_expr(builder, &args[0])?;

        // Extend the length to 64-bits.
        builder.unop(UnaryOp::I64ExtendUI32);

        // Traverse the index, leaving the value on top of the stack.
        generator.traverse_expr(builder, &args[1])?;

        // Check if the upper 64-bits are greater than 0.
        builder.i64_const(0).binop(BinaryOp::I64GtU);

        // Save the overflow indicator to a local.
        let overflow_local = generator.module.locals.add(ValType::I32);
        builder.local_set(overflow_local);

        // Save the lower part of the index to a local.
        let index_local = generator.module.locals.add(ValType::I64);
        builder.local_tee(index_local);

        // Check if the lower 64-bits are greater than 1024x1024 (max value
        // size). We do this check before comparing with the length of the list
        // because it ensures that the multiplication will not overflow.
        builder.i64_const(1024 * 1024).binop(BinaryOp::I64GtU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

        // Push the index onto the stack again.
        builder.local_get(index_local);

        // Record the element type, for use later. `None` indicates that the
        // element type is just a byte (for `string-ascii` or `buff`).
        let mut element_ty = None;

        // Get the offset of the specified index.
        generator
            .get_expr_type(&args[0])
            .ok_or_else(|| GeneratorError::InternalError("append result must be typed".to_string()))
            .and_then(|ty| match ty {
                TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                    // The length of the list in bytes is on the top of the stack. If we
                    // divide that by the length of each element, then we'll have the
                    // length of the list in elements.
                    let elem_ty = list.get_list_item_type().clone();
                    let element_length = get_type_size(&elem_ty);
                    builder.i64_const(element_length as i64);

                    // Save the element type for later.
                    element_ty = Some(elem_ty.clone());

                    // Multiply the index by the length of each element to get
                    // byte-offset into the list.
                    builder.binop(BinaryOp::I64Mul);

                    Ok(())
                }
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => {
                    // The index is the same as the byte-offset, so just leave
                    // it as-is.
                    Ok(())
                }
                TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    _,
                ))) => Err(GeneratorError::NotImplemented),
                _ => Err(GeneratorError::InternalError(
                    "expected sequence type".to_string(),
                )),
            })?;

        // Save the element offset to the local.
        builder.local_tee(index_local);

        // Check if the element offset is out of range by comparing it to the
        // length of the list.
        builder.binop(BinaryOp::I64LeU);

        // Or with the overflow indicator.
        builder.local_get(overflow_local).binop(BinaryOp::I32Or);

        let placeholder_ty = element_ty.clone();

        // If the index is out of range, then return `none`, else load the
        // value at the specified index and return `(some value)`.
        let result_ty = generator.get_expr_type(expr).ok_or_else(|| {
            GeneratorError::InternalError("append result must be typed".to_string())
        })?;
        let result_wasm_types = clar2wasm_ty(result_ty);
        builder.if_else(
            InstrSeqType::new(
                &mut generator.module.types,
                &[ValType::I32],
                &result_wasm_types,
            ),
            |then| {
                // First, drop the offset.
                then.drop();

                // Push the `none` indicator.
                then.i32_const(0);

                // Then push a placeholder for the element type.
                if let Some(elem_ty) = placeholder_ty {
                    // Read the element type from the list.
                    add_placeholder_for_clarity_type(then, &elem_ty)
                } else {
                    // The element type is an in-memory type, so we need
                    // placeholders for offset and length
                    then.i32_const(0).i32_const(0);
                }
            },
            |else_| {
                let offset_local = generator.module.locals.add(ValType::I32);

                // Add the element offset to the offset of the list.
                else_
                    .local_get(index_local)
                    // We know this offset is in range, so it must be a 32-bit
                    // value, so this operation is safe.
                    .unop(UnaryOp::I32WrapI64)
                    .binop(BinaryOp::I32Add)
                    .local_set(offset_local);

                // Push the `some` indicator
                else_.i32_const(1);

                // Load the value at the specified offset.
                if let Some(elem_ty) = &element_ty {
                    generator.read_from_memory(else_, offset_local, 0, elem_ty);
                } else {
                    // The element type is a byte (from a string or buffer), so
                    // we need to push the offset and length (1) to the
                    // stack.
                    else_.local_get(offset_local).i32_const(1);
                }
            },
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct ReplaceAt;

impl Word for ReplaceAt {
    fn name(&self) -> ClarityName {
        "replace-at?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let seq_ty = generator
            .get_expr_type(args.get_expr(0)?)
            .ok_or(GeneratorError::InternalError(
                "replace-at? result must be typed".to_string(),
            ))?
            .clone();

        // Create a new stack local for a copy of the input list
        let (dest_offset, length) =
            generator.create_call_stack_local(builder, &seq_ty, false, true);

        // Put the destination offset on the stack
        builder.local_get(dest_offset);

        // Traverse the list, leaving the offset and length on top of the stack.
        generator.traverse_expr(builder, args.get_expr(0)?)?;

        // Copy the input list to the new stack local
        let memory = generator.get_memory();
        builder.memory_copy(memory, memory);

        // Traverse the replacement value, leaving it on the stack.
        generator.traverse_expr(builder, args.get_expr(2)?)?;

        // Extend the sequence length to 64-bits.
        builder.i32_const(length).unop(UnaryOp::I64ExtendUI32);

        // Traverse the index, leaving the value on top of the stack.
        generator.traverse_expr(builder, args.get_expr(1)?)?;

        // Check if the upper 64-bits are greater than 0.
        builder.i64_const(0).binop(BinaryOp::I64GtU);

        // Save the overflow indicator to a local.
        let overflow_local = generator.module.locals.add(ValType::I32);
        builder.local_set(overflow_local);

        // Save the lower part of the index to a local.
        let index_local = generator.module.locals.add(ValType::I64);
        builder.local_tee(index_local);

        // Check if the lower 64-bits are greater than 1024x1024 (max value
        // size). We do this check before comparing with the length of the list
        // because it ensures that the multiplication will not overflow.
        builder.i64_const(1024 * 1024).binop(BinaryOp::I64GtU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

        // Push the index onto the stack again.
        builder.local_get(index_local);

        // Record the element type, for use later. `None` indicates that the
        // element type is just a byte (for `string-ascii` or `buff`).
        let mut element_ty = None;

        // Get the offset of the specified index.
        match &seq_ty {
            TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                // The length of the list in bytes is on the top of the stack. If we
                // divide that by the length of each element, then we'll have the
                // length of the list in elements.
                let elem_ty = list.get_list_item_type().clone();
                let element_length = get_type_size(&elem_ty);
                builder.i64_const(element_length as i64);

                // Save the element type for later.
                element_ty = Some(elem_ty.clone());

                // Multiply the index by the length of each element to get
                // byte-offset into the list.
                builder.binop(BinaryOp::I64Mul);

                Ok(())
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
            | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                // The index is the same as the byte-offset, so just leave
                // it as-is.
                Ok(())
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                Err(GeneratorError::NotImplemented)
            }
            _ => Err(GeneratorError::InternalError(
                "expected sequence type".to_string(),
            )),
        }?;

        // Save the element offset to the local.
        builder.local_tee(index_local);

        // Check if the element offset is out of range by comparing it to the
        // length of the list.
        builder.binop(BinaryOp::I64LeU);

        // Or with the overflow indicator.
        builder.local_get(overflow_local).binop(BinaryOp::I32Or);

        let input_ty = generator.get_expr_type(args.get_expr(2)?).ok_or_else(|| {
            GeneratorError::InternalError("replace-at? value must be typed".to_string())
        })?;
        let input_wasm_types = clar2wasm_ty(input_ty);

        let drop_ty = element_ty.clone();

        // If the index is out of range, then return `none`, else load the
        // value at the specified index and return `(some value)`.
        let result_ty = generator.get_expr_type(expr).ok_or_else(|| {
            GeneratorError::InternalError("append result must be typed".to_string())
        })?;
        let result_wasm_types = clar2wasm_ty(result_ty);
        builder.if_else(
            InstrSeqType::new(
                &mut generator.module.types,
                &input_wasm_types,
                &result_wasm_types,
            ),
            |then| {
                // First, drop the value.
                if let Some(drop_ty) = drop_ty {
                    drop_value(then, &drop_ty);
                } else {
                    // The value is a byte, but it's represented by an offset
                    // and length, so drop those.
                    then.drop().drop();
                }

                // Push the `none` indicator and placeholders for offset/length
                then.i32_const(0).i32_const(0).i32_const(0);
            },
            |else_| {
                let offset_local = generator.module.locals.add(ValType::I32);

                // Add the element offset to the offset of the destination.
                else_
                    .local_get(index_local)
                    // We know this offset is in range, so it must be a 32-bit
                    // value, so this operation is safe.
                    .unop(UnaryOp::I32WrapI64)
                    .local_get(dest_offset)
                    .binop(BinaryOp::I32Add)
                    .local_set(offset_local);

                // Write the value to the specified offset.
                if let Some(elem_ty) = &element_ty {
                    generator.write_to_memory(else_, offset_local, 0, elem_ty);
                } else {
                    // The element type is a byte (from a string or buffer), so
                    // we need to just copy that byte to the specified offset.

                    // Drop the length of the value (it must be 1)
                    else_.drop();

                    // Save the source offset to a local.
                    let src_local = generator.module.locals.add(ValType::I32);
                    else_.local_set(src_local);

                    else_
                        .local_get(offset_local)
                        .local_get(src_local)
                        .i32_const(1)
                        .memory_copy(memory, memory);
                }

                // Push the `some` indicator with destination offset/length.
                else_.i32_const(1).local_get(dest_offset).i32_const(length);
            },
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct Slice;

impl Word for Slice {
    fn name(&self) -> ClarityName {
        "slice?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let seq = args.get_expr(0)?;

        // Traverse the sequence, leaving the offset and length on the stack.
        generator.traverse_expr(builder, seq)?;

        // Extend the sequence length to 64-bits.
        builder.unop(UnaryOp::I64ExtendUI32);

        // Save the length to a local.
        let length_local = generator.module.locals.add(ValType::I64);
        builder.local_tee(length_local);

        // Traverse the left position, leaving it on the stack.
        generator.traverse_expr(builder, args.get_expr(1)?)?;

        // Check if the upper 64-bits are greater than 0.
        builder.i64_const(0).binop(BinaryOp::I64GtU);

        // Save the overflow indicator to a local.
        let overflow_local = generator.module.locals.add(ValType::I32);
        builder.local_set(overflow_local);

        // Save the lower part of the index, which will ultimately be
        // multiplied by the element size and added to the source offset to be
        // the offset of the result, to a local.
        let left_local = generator.module.locals.add(ValType::I64);
        builder.local_tee(left_local);

        // Check if the lower 64-bits are greater than 1024x1024 (max value
        // size). We do this check before comparing with the length of the list
        // because it ensures that the multiplication will not overflow.
        builder.i64_const(1024 * 1024).binop(BinaryOp::I64GtU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

        // Push the lower bound index onto the stack again.
        builder.local_get(left_local);

        let seq_ty = generator
            .get_expr_type(seq)
            .ok_or(GeneratorError::InternalError(
                "slice? sequence must be typed".to_string(),
            ))?
            .clone();

        // Get the offset of the specified index.
        match &seq_ty {
            TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                // The length of the list in bytes is on the top of the stack. If we
                // divide that by the length of each element, then we'll have the
                // length of the list in elements.
                let elem_ty = list.get_list_item_type().clone();
                let element_length = get_type_size(&elem_ty);
                builder.i64_const(element_length as i64);

                // Multiply the index by the length of each element to get
                // byte-offset into the list.
                builder.binop(BinaryOp::I64Mul);

                Ok(())
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
            | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                // The index is the same as the byte-offset, so just leave
                // it as-is.
                Ok(())
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                Err(GeneratorError::NotImplemented)
            }
            _ => Err(GeneratorError::InternalError(
                "expected sequence type".to_string(),
            )),
        }?;

        // Save the element offset to the local.
        builder.local_tee(left_local);

        // Check if the element offset is out of range by comparing it to the
        // length of the list.
        builder.binop(BinaryOp::I64LeU);

        // Or with the overflow indicator.
        builder.local_get(overflow_local).binop(BinaryOp::I32Or);

        // Save the overflow indicator to a local.
        builder.local_set(overflow_local);

        // Extend the base offset to 64-bits and save it to a local.
        let base_offset_local = generator.module.locals.add(ValType::I64);
        builder
            .unop(UnaryOp::I64ExtendUI32)
            .local_tee(base_offset_local);

        // Add this left offset to the offset of the list, which is on the top
        // of the stack now, to use as the offset of the slice, if it is in
        // bounds.
        // If it is in bounds, then this truncation to 32-bits will be safe.
        builder
            .local_get(left_local)
            .binop(BinaryOp::I64Add)
            .local_set(left_local);

        // Now check the right bound.

        // First, reload the source length.
        builder.local_get(length_local);

        // Traverse the right position, leaving it on the stack.
        generator.traverse_expr(builder, args.get_expr(2)?)?;

        // Check if the upper 64-bits are greater than 0.
        builder.i64_const(0).binop(BinaryOp::I64GtU);

        // Save the overflow indicator to a local.
        let overflow_local = generator.module.locals.add(ValType::I32);
        builder.local_set(overflow_local);

        // Save the lower part of the index, which will ultimately be
        // multiplied by the element size and added to the source offset to be
        // the offset of the result, to a local.
        let right_local = generator.module.locals.add(ValType::I64);
        builder.local_tee(right_local);

        // Check if the lower 64-bits are greater than 1024x1024 (max value
        // size). We do this check before comparing with the length of the list
        // because it ensures that the multiplication will not overflow.
        builder.i64_const(1024 * 1024).binop(BinaryOp::I64GtU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

        // Push the lower bound index onto the stack again.
        builder.local_get(right_local);

        let seq_ty = generator
            .get_expr_type(seq)
            .ok_or(GeneratorError::InternalError(
                "slice? sequence must be typed".to_string(),
            ))?
            .clone();

        // Get the offset of the specified index.
        match &seq_ty {
            TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                // The length of the list in bytes is on the top of the stack. If we
                // divide that by the length of each element, then we'll have the
                // length of the list in elements.
                let elem_ty = list.get_list_item_type().clone();
                let element_length = get_type_size(&elem_ty);
                builder.i64_const(element_length as i64);

                // Multiply the index by the length of each element to get
                // byte-offset into the list.
                builder.binop(BinaryOp::I64Mul);

                Ok(())
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
            | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                // The index is the same as the byte-offset, so just leave
                // it as-is.
                Ok(())
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                Err(GeneratorError::NotImplemented)
            }
            _ => Err(GeneratorError::InternalError(
                "expected sequence type".to_string(),
            )),
        }?;

        // Save the element offset to the local.
        builder.local_tee(right_local);

        // Check if the element offset is out of range by comparing it to the
        // length of the list.
        builder.binop(BinaryOp::I64LtU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

        // Add the right offset to the offset of the list, which is on the top
        // of the stack now, to get the end of the slice, if it is in bounds.
        // If it is in bounds, then this truncation to 32-bits will be safe.
        builder
            .local_get(base_offset_local)
            .local_get(right_local)
            .binop(BinaryOp::I64Add)
            .local_set(right_local);

        // Push a `0` and a `1` to the stack, for none or some, to be selected
        // by the `select` instruction, using the overflow indicator.
        builder.i32_const(0).i32_const(1).local_get(overflow_local);

        // If either bound is out of range, then return `none`, else return
        // `(some sequence)`, where `sequence` is the slice of the input
        // sequence with offset left and length right - left.
        builder.select(Some(ValType::I32));

        // Now push the offset (`local_left`) and length
        // (`local_right - local_left`). If the result is `none`, then these
        // will just be ignored. If the offsets are in range, then the
        // truncation to 32-bits is safe.
        builder
            .local_get(left_local)
            .unop(UnaryOp::I32WrapI64)
            .local_get(right_local)
            .local_get(left_local)
            .binop(BinaryOp::I64Sub)
            .unop(UnaryOp::I32WrapI64);

        Ok(())
    }
}
