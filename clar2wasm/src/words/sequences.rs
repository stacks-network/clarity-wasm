use clarity::vm::clarity_wasm::get_type_size;
use clarity::vm::types::{
    FunctionType, ListTypeData, SequenceSubtype, StringSubtype, TypeSignature,
};
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ir::{self, BinaryOp, IfElse, InstrSeqType, Loop, UnaryOp};
use walrus::ValType;

use crate::check_args;
use crate::error_mapping::ErrorMap;
use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, type_from_sequence_element,
    ArgumentsExt, GeneratorError, SequenceElementType, WasmGenerator,
};
use crate::wasm_utils::{check_argument_count, ArgumentCountCheck};
use crate::words::{self, ComplexWord};

#[derive(Debug)]
pub struct ListCons;

impl ComplexWord for ListCons {
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
            .ok_or_else(|| GeneratorError::TypeError("list expression must be typed".to_owned()))?
            .clone();
        let (elem_ty, _num_elem) =
            if let TypeSignature::SequenceType(SequenceSubtype::ListType(list_type)) = &ty {
                (list_type.get_list_item_type(), list_type.get_max_len())
            } else {
                return Err(GeneratorError::TypeError(format!(
                    "Expected list type for list expression, but found: {:?}",
                    ty
                )));
            };

        // Allocate space on the data stack for the entire list
        let (offset, _size) = generator.create_call_stack_local(builder, &ty, false, true);

        // Loop through the expressions in the list and store them onto the
        // data stack.
        let mut total_size = 0;
        for expr in list.iter() {
            // WORKAROUND: if you have a list like `(list (some 1) none)`, even if the list elements have type
            // `optional int`, the typechecker will give NoType to `none`.
            // This means that the placeholder will be represented with a different number of `ValType`, and will
            // cause errors (example: function called with wrong number of arguments).
            // While we wait for a real fix in the typechecker, here is a workaround to set all the elements types.
            generator.set_expr_type(expr, elem_ty.clone())?;

            generator.traverse_expr(builder, expr)?;
            // Write this element to memory
            let elem_size = generator.write_to_memory(builder, offset, total_size, elem_ty)?;
            total_size += elem_size;
        }

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(total_size as i32);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Fold;

impl ComplexWord for Fold {
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
        check_args!(generator, builder, 3, args.len(), ArgumentCountCheck::Exact);

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

        // WORKAROUND: Get the type of the function being called, and set the
        // type of the initial value to match the functions parameter type.
        // This is a workaround for the typechecker not being able to infer
        // the complete type of initial value.
        if let Some(FunctionType::Fixed(fixed)) = generator.get_function_type(func) {
            let initial_ty = fixed
                .args
                .get(1)
                .ok_or_else(|| {
                    GeneratorError::TypeError("expected function with 2 arguments".into())
                })?
                .signature
                .clone();
            generator.set_expr_type(initial, initial_ty)?;
        }

        // The result type must match the type of the initial value
        let result_clar_ty = generator
            .get_expr_type(initial)
            .ok_or_else(|| {
                GeneratorError::TypeError(
                    "fold's initial value expression must be typed".to_owned(),
                )
            })?
            .clone();
        let result_wasm_types = clar2wasm_ty(&result_clar_ty);

        // Get the type of the sequence
        let elem_ty = generator.get_sequence_element_type(sequence)?;

        // Evaluate the sequence, which will load it into the call stack,
        // leaving the offset and size on the data stack.
        generator.traverse_expr(builder, sequence)?;
        // STACK: [offset, length]

        let length = generator.module.locals.add(ValType::I32);
        let offset = generator.module.locals.add(ValType::I32);
        let end_offset = generator.module.locals.add(ValType::I32);

        // Store the length and offset into locals.
        builder.local_set(length).local_tee(offset);
        // STACK: [offset]

        // Compute the ending offset of the sequence.
        builder
            .local_get(length)
            .binop(BinaryOp::I32Add)
            .local_set(end_offset);
        // STACK: []

        // Evaluate the initial value, so that its result is on the data stack
        generator.traverse_expr(builder, initial)?;
        // STACK: [initial_val]

        // If the length of the sequence is 0, then just return the initial
        // value which is already on the stack. Else, loop over the sequence
        // and apply the function.
        let then = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &result_wasm_types,
            &result_wasm_types,
        ));
        let then_id = then.id();

        let mut else_ = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &result_wasm_types,
            &result_wasm_types,
        ));
        let else_id = else_.id();

        // Define local(s) to hold the intermediate result, and initialize them
        // with the initial value. Note that we are looping in reverse order,
        // to pop values from the top of the stack.
        let result_locals = generator.save_to_locals(&mut else_, &result_clar_ty, true);

        // Define the body of a loop, to loop over the sequence and make the
        // function call.
        let mut loop_ = else_.dangling_instr_seq(None);
        let loop_id = loop_.id();

        // Load the element from the sequence
        let elem_size = match &elem_ty {
            SequenceElementType::Other(elem_ty) => {
                generator.read_from_memory(&mut loop_, offset, 0, elem_ty)?
            }
            SequenceElementType::Byte => {
                // The element type is a byte, so we can just push the
                // offset and length (1) to the stack.
                loop_.local_get(offset).i32_const(1);
                1
            }
            SequenceElementType::UnicodeScalar => {
                // The element type is a 32-bit unicode scalar, so we can just push the
                // offset and length (4) to the stack.
                loop_.local_get(offset).i32_const(4);
                4
            }
        };

        // Push the locals to the stack
        for result_local in &result_locals {
            loop_.local_get(*result_local);
        }

        if let Some(simple) = words::lookup_simple(func).or(words::lookup_variadic_simple(func)) {
            // Call simple builtin

            let arg_a_ty = type_from_sequence_element(&elem_ty);
            let arg_types = &[arg_a_ty, result_clar_ty.clone()];

            simple.visit(generator, &mut loop_, arg_types, &result_clar_ty)?;
        } else {
            // Call user defined function
            generator.visit_call_user_defined(&mut loop_, &result_clar_ty, func)?;
        }
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

        else_.instr(Loop { seq: loop_id });

        // Push the locals to the stack
        for result_local in result_locals {
            else_.local_get(result_local);
        }

        builder
            .local_get(length)
            .unop(UnaryOp::I32Eqz)
            .instr(IfElse {
                consequent: then_id,
                alternative: else_id,
            });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Append;

impl ComplexWord for Append {
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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("append result must be typed".to_string()))?
            .clone();

        let list = args.get_expr(0)?;
        let elem = args.get_expr(1)?;

        // WORKAROUND: setting correct types for arguments
        match &ty {
            TypeSignature::SequenceType(SequenceSubtype::ListType(ltd)) => {
                generator.set_expr_type(
                    list,
                    #[allow(clippy::expect_used)]
                    ListTypeData::new_list(ltd.get_list_item_type().clone(), ltd.get_max_len() - 1)
                        .expect("Argument type should be correct as it is the same as the expression type with a smaller max_len")
                        .into(),
                )?;
                generator.set_expr_type(elem, ltd.get_list_item_type().clone())?;
            }
            _ => {
                return Err(GeneratorError::TypeError(
                    "append result should be a list".to_owned(),
                ))
            }
        }

        let memory = generator.get_memory()?;

        // Allocate stack space for the new list.
        let (write_ptr, length) = generator.create_call_stack_local(builder, &ty, false, true);

        // Push the offset and length of this list to the stack to be returned.
        builder.local_get(write_ptr).i32_const(length);

        // Push the write pointer onto the stack for `memory.copy`.
        builder.local_get(write_ptr);

        // Traverse the list to append to, leaving the offset and length on
        // top of the stack.
        generator.traverse_expr(builder, list)?;

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
        generator.traverse_expr(builder, elem)?;

        // Get the type of the element that we're appending.
        let elem_ty = generator
            .get_expr_type(elem)
            .ok_or_else(|| GeneratorError::TypeError("append element must be typed".to_string()))?
            .clone();

        // Store the element at the write pointer.
        generator.write_to_memory(builder, write_ptr, 0, &elem_ty)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct AsMaxLen;

impl ComplexWord for AsMaxLen {
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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        // Push a `0` and a `1` to the stack, to be used by the `select`
        // instruction later.
        builder.i32_const(0).i32_const(1);

        // Traverse the input list, leaving the offset and length on top of
        // the stack.
        let seq = args.get_expr(0)?;
        generator.traverse_expr(builder, seq)?;

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
            .get_expr_type(seq)
            .ok_or_else(|| GeneratorError::TypeError("append result must be typed".to_string()))
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
                ))) => {
                    builder.i32_const(4);

                    // Divide the length of the list by the length of each element to get
                    // the number of elements in the list.
                    builder.binop(BinaryOp::I32DivU);

                    Ok(())
                }
                // The byte length of buffers and ASCII strings is the same as
                // the value length, so just leave it as-is.
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => Ok(()),
                _ => Err(GeneratorError::TypeError(
                    "expected sequence type".to_string(),
                )),
            })?;

        // Convert this 32-bit length to a 64-bit value, for comparison.
        builder.unop(UnaryOp::I64ExtendUI32);

        // Traverse the second argument, the desired length, leaving the low
        // and high parts on the stack, then drop the high part.
        generator.traverse_expr(builder, args.get_expr(1)?)?;
        builder.drop();

        // Compare the length of the list to the desired length.
        builder.binop(BinaryOp::I64GtU);

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

impl ComplexWord for Concat {
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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let memory = generator.get_memory()?;

        // Create a new sequence to hold the result in the stack frame
        let ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("concat expression must be typed".to_owned()))?
            .clone();
        let (offset, _) = generator.create_call_stack_local(builder, &ty, false, true);

        builder.local_get(offset);

        // Traverse the lhs, leaving it on the data stack (offset, size)
        let lhs = args.get_expr(0)?;
        // WORKAROUND: typechecker issue for lists
        generator.set_expr_type(lhs, ty.clone())?;
        generator.traverse_expr(builder, lhs)?;

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
        let rhs = args.get_expr(1)?;
        // WORKAROUND: typechecker issue for lists
        generator.set_expr_type(rhs, ty.clone())?;
        generator.traverse_expr(builder, rhs)?;

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
pub struct Map;

impl ComplexWord for Map {
    fn name(&self) -> ClarityName {
        "map".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(
            generator,
            builder,
            2,
            args.len(),
            ArgumentCountCheck::AtLeast
        );

        let fname = args.get_name(0)?;

        let seq_ty = generator
            .get_expr_type(args.get_expr(1)?)
            .ok_or_else(|| GeneratorError::TypeError("list expression must be typed".to_owned()))?
            .clone();

        // WORKAROUND: Get the type of the function being called, and set the
        // type of the sequence value to match the functions parameter type.
        // This is a workaround for the typechecker not being able to infer
        // the complete type of initial value.
        if let TypeSignature::SequenceType(SequenceSubtype::ListType(lt)) = &seq_ty {
            let size = get_type_size(lt.get_list_item_type()) as u32;

            if let Some(FunctionType::Fixed(fixed)) = generator.get_function_type(fname) {
                let function_ty = fixed
                    .args
                    .first()
                    .ok_or_else(|| {
                        GeneratorError::TypeError("expected function with 2 arguments".into())
                    })?
                    .signature
                    .clone();

                match ListTypeData::new_list(function_ty, size) {
                    Ok(list_type_data) => {
                        generator.set_expr_type(
                            args.get_expr(1)?,
                            TypeSignature::SequenceType(SequenceSubtype::ListType(list_type_data)),
                        )?;
                    }
                    Err(_) => {
                        return Err(GeneratorError::TypeError(
                            "Failed to workaround and create a list type".into(),
                        ));
                    }
                }
            }
        }

        let ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("list expression must be typed".to_owned()))?
            .clone();

        let return_element_type =
            if let TypeSignature::SequenceType(SequenceSubtype::ListType(list_type)) = &ty {
                list_type.get_list_item_type()
            } else {
                return Err(GeneratorError::TypeError(format!(
                    "Expected list type for list expression, but found: {:?}",
                    ty
                )));
            };

        let return_element_size = get_type_size(return_element_type);

        let min_num_elements = generator.module.locals.add(ValType::I32);
        builder.i32_const(i32::MAX);
        builder.local_set(min_num_elements);

        let mut input_offsets = vec![];
        let mut input_element_types = vec![];
        let mut input_element_sizes = vec![];
        let mut input_num_elements = vec![];

        for arg in args.iter().skip(1) {
            // get the type of the seq, and the sizes.

            let (element_ty, element_size) = match generator
                .get_expr_type(arg)
                .ok_or_else(|| {
                    GeneratorError::TypeError("sequence expression must be typed".to_owned())
                })?
                .clone()
            {
                TypeSignature::SequenceType(SequenceSubtype::ListType(lt)) => {
                    let element_ty = lt.get_list_item_type().clone();
                    let element_size = get_type_size(&element_ty);

                    (SequenceElementType::Other(element_ty), element_size)
                }
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => (SequenceElementType::Byte, 1),
                TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    _,
                ))) => (SequenceElementType::UnicodeScalar, 4),
                _ => {
                    return Err(GeneratorError::TypeError(
                        "expected sequence type".to_string(),
                    ));
                }
            };

            input_element_types.push(element_ty);
            input_element_sizes.push(element_size);

            generator.traverse_expr(builder, arg)?;
            // [ offset, length ]
            builder.i32_const(element_size);
            // [ offset, length, element_size ]
            builder.binop(ir::BinaryOp::I32DivS);
            // [ offset, num_elements ]

            let num_elements = generator.module.locals.add(ValType::I32);
            builder.local_tee(num_elements);
            builder.local_get(num_elements);
            // [ offset, num_elements, num_elements ]
            input_num_elements.push(num_elements);

            builder.local_get(min_num_elements);
            // [ offset, num_elements, num_elements, min_num_elements ]

            builder.binop(ir::BinaryOp::I32LeS);
            // [ offset, num_elements, is_less ]

            builder.if_else(
                InstrSeqType::new(&mut generator.module.types, &[ValType::I32], &[]),
                |t| {
                    t.local_set(min_num_elements);
                },
                |e| {
                    e.drop();
                },
            );
            // [ offset ]

            let offset = generator.module.locals.add(ValType::I32);
            builder.local_set(offset);
            // [ ]
            input_offsets.push(offset);
        }

        // Allocate worst case size to ensure enough stack space is reserved at compile time
        let (output_base, _) = generator.create_call_stack_local(builder, &ty, false, true);

        // Allocate space on the call stack for the output list.
        let output_offset = generator.module.locals.add(ValType::I32);
        builder.local_get(output_base).local_set(output_offset);

        // Create an index to count the number of elements to loop over.
        let index = generator.module.locals.add(ValType::I32);
        builder.i32_const(0).local_set(index);

        // Loop over the min_num_elements of the input sequences, calling the
        // function on each set of elements. The result of the function call
        // will be written to the output sequence. The loop_exit block allows
        // us to put the condition at the top of the loop.
        let mut loop_exit = builder.dangling_instr_seq(None);
        let loop_exit_id = loop_exit.id();
        let mut loop_ = loop_exit.dangling_instr_seq(None);
        let loop_id = loop_.id();

        // See if we're calling a simple function, and if it's variadic

        let mut simple = words::lookup_simple(fname);
        let mut variadic = false;

        if simple.is_none() {
            if let Some(simple_variadic) = words::lookup_variadic_simple(fname) {
                variadic = true;
                simple = Some(simple_variadic)
            }
        }

        let arg_types: Vec<_> = input_element_types
            .iter()
            .map(type_from_sequence_element)
            .collect();

        // Check if we've reached the min_num_elements
        loop_
            .local_get(index)
            .local_get(min_num_elements)
            .binop(BinaryOp::I32GeU)
            .br_if(loop_exit_id);

        // For each input sequence, load the next element, and adjust the
        // offset for the next iteration.
        for (i, offset) in input_offsets.iter().enumerate() {
            match &input_element_types[i] {
                SequenceElementType::Other(elem_ty) => {
                    generator.read_from_memory(&mut loop_, *offset, 0, elem_ty)?;
                }
                SequenceElementType::Byte => {
                    // The element type is a byte, so we can just push the
                    // offset and length (1) to the stack.
                    loop_.local_get(*offset).i32_const(1);
                }
                SequenceElementType::UnicodeScalar => {
                    // The element type is a 32-bit unicode scalar, so we can just push the
                    // offset and length (4) to the stack.
                    loop_.local_get(*offset).i32_const(4);
                }
            }

            // If we have variadics, we need to interleave the calls
            // if the arg length is 1, this is a no-op
            if let Some(simple) = simple {
                if variadic && i > 0 {
                    simple.visit(
                        generator,
                        &mut loop_,
                        &arg_types[i - 1..=i],
                        return_element_type,
                    )?;
                }
            }

            // Increment the offset by the size of the element.
            loop_
                .local_get(*offset)
                .i32_const(input_element_sizes[i])
                .binop(BinaryOp::I32Add)
                .local_set(*offset);
        }

        if let Some(simple) = simple {
            // If not variadic, _or_ if the arg length is one (unary operations)
            if !variadic || arg_types.len() == 1 {
                simple.visit(generator, &mut loop_, &arg_types, return_element_type)?;
            }
        } else {
            // Call user defined function.
            generator.visit_call_user_defined(&mut loop_, return_element_type, fname)?;
        }

        // Write the result to the output sequence.
        generator.write_to_memory(&mut loop_, output_offset, 0, return_element_type)?;

        // Increment the output offset by the size of the element.
        loop_
            .local_get(output_offset)
            .i32_const(return_element_size)
            .binop(BinaryOp::I32Add)
            .local_set(output_offset);

        // Increment the index.
        loop_
            .local_get(index)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_tee(index);

        // Loop back to the top.
        loop_.br(loop_id);

        // Add the loop to the loop_exit block.
        loop_exit.instr(Loop { seq: loop_id });

        // Add the loop_exit block to the main block.
        builder.instr(walrus::ir::Block { seq: loop_exit_id });

        builder
            .local_get(output_base)
            .local_get(min_num_elements)
            .i32_const(return_element_size)
            .binop(ir::BinaryOp::I32Mul);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Len;

impl ComplexWord for Len {
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
        check_args!(generator, builder, 1, args.len(), ArgumentCountCheck::Exact);

        // Traverse the sequence, leaving the offset and length on the stack.
        let seq = args.get_expr(0)?;
        generator.traverse_expr(builder, seq)?;

        // Save the length, then drop the offset and push the length back.
        let length_local = generator.module.locals.add(ValType::I32);
        builder
            .local_set(length_local)
            .drop()
            .local_get(length_local);

        // Get the length
        generator
            .get_expr_type(seq)
            .ok_or_else(|| GeneratorError::TypeError("append result must be typed".to_string()))
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
                ))) => {
                    // UTF8 is represented as 32-bit unicode scalars values.
                    builder.i32_const(4);
                    builder.binop(BinaryOp::I32DivU);

                    Ok(())
                }
                // The byte length of buffers and ASCII strings is the same as
                // the value length, so just leave it as-is.
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => Ok(()),
                _ => Err(GeneratorError::TypeError(
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

impl ComplexWord for ElementAt {
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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        // Traverse the sequence, leaving the offset and length on the stack.
        let seq = args.get_expr(0)?;
        generator.traverse_expr(builder, seq)?;

        // Extend the length to 64-bits.
        builder.unop(UnaryOp::I64ExtendUI32);

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

        // Record the element type, for use later.
        let element_ty: SequenceElementType = generator
            .get_expr_type(seq)
            .ok_or_else(|| GeneratorError::TypeError("append result must be typed".to_string()))
            .and_then(|ty| match ty {
                TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                    // The length of the list in bytes is on the top of the stack. If we
                    // divide that by the length of each element, then we'll have the
                    // length of the list in elements.
                    let elem_ty = list.get_list_item_type();
                    let element_length = get_type_size(elem_ty);
                    builder.i64_const(element_length as i64);

                    // Multiply the index by the length of each element to get
                    // byte-offset into the list.
                    builder.binop(BinaryOp::I64Mul);

                    Ok(SequenceElementType::Other(elem_ty.clone()))
                }
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
                | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                    _,
                ))) => {
                    // The index is the same as the byte-offset, so just leave
                    // it as-is.
                    Ok(SequenceElementType::Byte)
                }
                TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                    _,
                ))) => {
                    // UTF8 is represented as 32-bit unicode scalars values.
                    builder.i64_const(4);
                    builder.binop(BinaryOp::I64Mul);

                    Ok(SequenceElementType::UnicodeScalar)
                }
                _ => Err(GeneratorError::TypeError(
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

        // If the index is out of range, then return `none`, else load the
        // value at the specified index and return `(some value)`.
        let result_ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("append result must be typed".to_string()))?;
        let result_wasm_types = clar2wasm_ty(result_ty);

        let branch_ty = InstrSeqType::new(
            &mut generator.module.types,
            &[ValType::I32],
            &result_wasm_types,
        );
        let mut then = builder.dangling_instr_seq(branch_ty);
        let then_id = then.id();

        // First, drop the offset.
        then.drop();

        // Push the `none` indicator.
        then.i32_const(0);

        // Then push a placeholder for the element type.
        match &element_ty {
            SequenceElementType::Byte | SequenceElementType::UnicodeScalar => {
                // The element type is an in-memory type, so we need
                // placeholders for offset and length
                then.i32_const(0).i32_const(0);
            }
            SequenceElementType::Other(elem_ty) => {
                // Read the element type from the list.
                add_placeholder_for_clarity_type(&mut then, elem_ty)
            }
        }

        let mut else_ = builder.dangling_instr_seq(branch_ty);
        let else_id = else_.id();

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
        match &element_ty {
            SequenceElementType::Byte => {
                // The element type is a byte (from a string or buffer), so
                // we need to push the offset and length (1) to the
                // stack.
                else_.local_get(offset_local).i32_const(1);
            }
            SequenceElementType::UnicodeScalar => {
                // UTF8 is represented as 32-bit unicode scalar values.
                else_.local_get(offset_local).i32_const(4);
            }
            SequenceElementType::Other(elem_ty) => {
                // If the element type is not UTF8, use `read_from_memory`.
                generator.read_from_memory(&mut else_, offset_local, 0, elem_ty)?;
            }
        }

        builder.instr(ir::IfElse {
            consequent: then_id,
            alternative: else_id,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct ReplaceAt;

impl ComplexWord for ReplaceAt {
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
        check_args!(generator, builder, 3, args.len(), ArgumentCountCheck::Exact);

        let seq = args.get_expr(0)?;
        let seq_ty = generator
            .get_expr_type(seq)
            .ok_or_else(|| {
                GeneratorError::TypeError("replace-at? result must be typed".to_string())
            })?
            .clone();

        // Create a new stack local for a copy of the input list
        let (dest_offset, length) =
            generator.create_call_stack_local(builder, &seq_ty, false, true);

        // Put the destination offset on the stack
        builder.local_get(dest_offset);

        // Traverse the list, leaving the offset and length on top of the stack.
        generator.traverse_expr(builder, seq)?;

        let memory = generator.get_memory()?;

        // Copy the input list to the new stack local
        builder.memory_copy(memory, memory);

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

        // Get the offset of the specified index.
        let element_ty = match &seq_ty {
            TypeSignature::SequenceType(SequenceSubtype::ListType(list)) => {
                // The length of the list in bytes is on the top of the stack. If we
                // divide that by the length of each element, then we'll have the
                // length of the list in elements.
                let elem_ty = list.get_list_item_type();
                let element_length = get_type_size(elem_ty);
                builder.i64_const(element_length as i64);

                // Multiply the index by the length of each element to get
                // byte-offset into the list.
                builder.binop(BinaryOp::I64Mul);

                Ok(SequenceElementType::Other(elem_ty.clone()))
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
            | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                // The index is the same as the byte-offset, so just leave
                // it as-is.

                Ok(SequenceElementType::Byte)
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                // UTF8 is represented as 32-bit unicode scalars values.
                builder.i64_const(4);
                builder.binop(BinaryOp::I64Mul);

                Ok(SequenceElementType::UnicodeScalar)
            }
            _ => Err(GeneratorError::TypeError(
                "expected sequence type".to_string(),
            )),
        }?;

        // Save the element offset to the local.
        builder.local_tee(index_local);

        // Check if the element offset is out of range by comparing it to the
        // length of the list.
        builder.binop(BinaryOp::I64LeU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

        // Traverse the replacement value, leaving it on the stack.
        let replacement = args.get_expr(2)?;
        generator.traverse_expr(builder, replacement)?;

        // For types `string-ascii`, `string-utf8` and `buff`, an empty replacement could be a
        // valid value with a max-len of 1. However, using one is a runtime error.
        if matches!(
            element_ty,
            SequenceElementType::Byte | SequenceElementType::UnicodeScalar
        ) {
            let repl_len = generator.module.locals.add(ValType::I32);
            builder.local_tee(repl_len).unop(UnaryOp::I32Eqz).if_else(
                None,
                |then| {
                    then.i32_const(ErrorMap::BadTypeConstruction as i32)
                        .call(generator.func_by_name("stdlib.runtime-error"));
                },
                |_| {},
            );
            builder.local_get(repl_len);
        }

        let input_ty = generator.get_expr_type(replacement).ok_or_else(|| {
            GeneratorError::TypeError("replace-at? value must be typed".to_string())
        })?;
        let input_wasm_types = clar2wasm_ty(input_ty);

        // Push the overflow result to the stack for `if_else`.
        builder.local_get(overflow_local);

        // If the index is out of range, then return `none`, else write the
        // value at the specified index and return `(some value)`.
        let result_ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("append result must be typed".to_string()))?;
        let result_wasm_types = clar2wasm_ty(result_ty);

        let mut then = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &input_wasm_types,
            &result_wasm_types,
        ));
        let then_id = then.id();

        // First, drop the value.
        match &element_ty {
            SequenceElementType::Other(elem_ty) => {
                // Read the element type from the list.
                drop_value(&mut then, elem_ty);
            }
            SequenceElementType::Byte | SequenceElementType::UnicodeScalar => {
                // The value is a byte or 32-bit scalar, but it's represented by an offset
                // and length, so drop those.
                then.drop().drop();
            }
        }

        // Push the `none` indicator and placeholders for offset/length
        then.i32_const(0).i32_const(0).i32_const(0);

        let mut else_ = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &input_wasm_types,
            &result_wasm_types,
        ));
        let else_id = else_.id();

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
        match &element_ty {
            SequenceElementType::Byte => {
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
            SequenceElementType::UnicodeScalar => {
                // The element is a 32-bit unicode scalar value, so we
                // need to just copy those 4 bytes to the specified offset.

                // Drop the length of the value (it must be 4)
                else_.drop();

                // Save the source offset to a local.
                let src_local = generator.module.locals.add(ValType::I32);
                else_.local_set(src_local);

                else_
                    .local_get(offset_local)
                    .local_get(src_local)
                    .i32_const(4)
                    .memory_copy(memory, memory);
            }
            SequenceElementType::Other(elem_ty) => {
                generator.write_to_memory(&mut else_, offset_local, 0, elem_ty)?;
            }
        }

        // Push the `some` indicator with destination offset/length.
        else_.i32_const(1).local_get(dest_offset).i32_const(length);

        builder.instr(IfElse {
            consequent: then_id,
            alternative: else_id,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Slice;

impl ComplexWord for Slice {
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
        check_args!(generator, builder, 3, args.len(), ArgumentCountCheck::Exact);

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
            .ok_or_else(|| GeneratorError::TypeError("slice? sequence must be typed".to_string()))?
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
                // UTF8 is represented as 32-bit unicode scalars values.
                builder.i64_const(4);
                builder.binop(BinaryOp::I64Mul);

                Ok(())
            }
            _ => Err(GeneratorError::TypeError(
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
            .ok_or_else(|| GeneratorError::TypeError("slice? sequence must be typed".to_string()))?
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
                // UTF8 is represented as 32-bit unicode scalars values.
                builder.i64_const(4);
                builder.binop(BinaryOp::I64Mul);

                Ok(())
            }
            _ => Err(GeneratorError::TypeError(
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

        // check if length is negative

        builder.local_get(right_local);
        builder.local_get(left_local);

        builder.binop(BinaryOp::I64LtU);

        // Or with the overflow indicator.
        builder
            .local_get(overflow_local)
            .binop(BinaryOp::I32Or)
            .local_set(overflow_local);

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

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::{crosscheck, crosscheck_compare_only, evaluate};

    #[test]
    fn fold_less_than_three_args() {
        let result = evaluate("(fold + (list 1 2 3))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 3 arguments, got 2"));
    }

    #[test]
    fn fold_more_than_three_args() {
        let result = evaluate("(fold + (list 1 2 3) 1 0)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 3 arguments, got 4"));
    }

    #[test]
    fn append_less_than_two_args() {
        let result = evaluate("(append (list 1 2 3))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn append_more_than_two_args() {
        let result = evaluate("(append (list 1 2 3) 1 0)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn as_max_len_less_than_two_args() {
        let result = evaluate("(as-max-len? (list 1 2 3))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn as_max_len_more_than_two_args() {
        let result = evaluate("(as-max-len? (list 1 2 3) 1 0)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn concat_less_than_two_args() {
        let result = evaluate("(concat (list 1 2 3))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn concat_more_than_two_args() {
        let result = evaluate("(concat (list 1 2 3) (list 4 5) (list 6 7))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn map_less_than_two_args() {
        let result = evaluate("(map +)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting >= 2 arguments, got 1"));
    }

    #[test]
    fn len_less_than_one_arg() {
        let result = evaluate("(len)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 0"));
    }

    #[test]
    fn len_more_than_one_arg() {
        let result = evaluate("(len (list 1 2 3) (list 4 5))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 2"));
    }

    #[test]
    fn element_at_less_than_two_args() {
        let result = evaluate("(element-at? (list 1 2 3))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn element_at_more_than_two_args() {
        let result = evaluate("(element-at? (list 1 2 3) 1 0)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn replace_at_less_than_three_args() {
        let result = evaluate("(replace-at? (list 1 2 3) 2)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 3 arguments, got 2"));
    }

    #[test]
    fn replace_at_more_than_three_args() {
        let result = evaluate("(replace-at? (list 1 2 3) 1 4 0)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 3 arguments, got 4"));
    }

    #[test]
    fn slice_less_than_three_args() {
        let result = evaluate("(slice? (list 1 2 3) u1)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 3 arguments, got 2"));
    }

    #[test]
    fn slice_more_than_three_args() {
        let result = evaluate("(slice? (list 1 2 3) u1 u2 u3)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 3 arguments, got 4"));
    }

    #[test]
    fn test_fold_sub() {
        crosscheck(
            r#"
(define-private (sub (x int) (y int))
    (- x y)
)
(fold sub (list 1 2 3 4) 0)
    "#,
            Ok(Some(Value::Int(2))),
        )
    }

    #[test]
    fn test_fold_builtin() {
        crosscheck(r#"(fold + (list 1 2 3 4) 0)"#, Ok(Some(Value::Int(10))))
    }

    #[test]
    fn test_fold_sub_empty() {
        crosscheck(
            r#"
(define-private (sub (x int) (y int))
    (- x y)
)
(define-private (fold-sub (l (list 10 int)))
    (fold sub l 42)
)
(fold-sub (list))
    "#,
            Ok(Some(Value::Int(42))),
        )
    }

    #[test]
    fn test_fold_string_ascii() {
        crosscheck(
            r#"
(define-private (concat-string (a (string-ascii 20)) (b (string-ascii 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-string "cdef" "ab")
    "#,
            Ok(Some(
                Value::string_ascii_from_bytes("fedcab".to_string().into_bytes()).unwrap(),
            )),
        )
    }

    #[test]
    fn test_fold_string_ascii_empty() {
        crosscheck(
            r#"
(define-private (concat-string (a (string-ascii 20)) (b (string-ascii 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-string "" "ab")
    "#,
            Ok(Some(
                Value::string_ascii_from_bytes("ab".to_string().into_bytes()).unwrap(),
            )),
        )
    }

    #[test]
    fn test_fold_string_utf8() {
        crosscheck(
            r#"
(define-private (concat-string (a (string-utf8 20)) (b (string-utf8 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-string u"cdef" u"ab")
    "#,
            Ok(Some(
                Value::string_utf8_from_bytes("fedcab".into()).unwrap(),
            )),
        )
    }

    #[test]
    fn test_fold_string_utf8_b() {
        crosscheck(
            r#"
(define-private (concat-string (a (string-utf8 20)) (b (string-utf8 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-string u"cdef" u"ab\u{1F98A}")
    "#,
            Ok(Some(
                Value::string_utf8_from_bytes("fedcab🦊".into()).unwrap(),
            )),
        )
    }

    #[test]
    fn test_fold_string_utf8_empty() {
        crosscheck(
            r#"
(define-private (concat-string (a (string-utf8 20)) (b (string-utf8 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-string u"" u"ab\u{1F98A}")
    "#,
            Ok(Some(Value::string_utf8_from_bytes("ab🦊".into()).unwrap())),
        )
    }

    #[test]
    fn test_fold_buffer() {
        crosscheck(
            r"
(define-private (concat-buff (a (buff 20)) (b (buff 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-buff 0x03040506 0x0102)
",
            Ok(Some(
                Value::buff_from(vec![0x06, 0x05, 0x04, 0x03, 0x01, 0x02]).unwrap(),
            )),
        )
    }

    #[test]
    fn test_fold_buffer_empty() {
        crosscheck(
            "
(define-private (concat-buff (a (buff 20)) (b (buff 20)))
    (unwrap-panic (as-max-len? (concat a b) u20))
)
(fold concat-buff 0x 0x0102)
",
            Ok(Some(Value::buff_from(vec![0x01, 0x02]).unwrap())),
        )
    }

    #[test]
    fn fold_init() {
        crosscheck(
            "(define-private (foo (index uint) (res (response bool uint)))
            (if (< index u1) (err u0) (ok true))
          )
          (define-private (bar)
            (fold foo (list u0) (ok true))
          )
          (bar)",
            Ok(Some(Value::err_uint(0))),
        );
    }

    #[test]
    fn test_map_simple_list() {
        crosscheck(
            r#"
(define-private (addify (a int))
    (+ a 1)
)
(map addify (list 1 2 3))
        "#,
            Ok(Some(
                Value::cons_list_unsanitized(vec![Value::Int(2), Value::Int(3), Value::Int(4)])
                    .unwrap(),
            )),
        )
    }

    #[test]
    fn test_map_simple_buff() {
        crosscheck(
            r#"
(define-private (zero-or-one (char (buff 1))) (if (is-eq char 0x00) 0x00 0x01))
(map zero-or-one 0x000102)
        "#,
            Ok(Some(
                Value::cons_list_unsanitized(vec![
                    Value::buff_from_byte(0),
                    Value::buff_from_byte(1),
                    Value::buff_from_byte(1),
                ])
                .unwrap(),
            )),
        )
    }

    #[test]
    fn test_map_simple_string_ascii() {
        crosscheck(
            r#"
(define-private (a-or-b (char (string-ascii 1))) (if (is-eq char "a") "a" "b"))
(map a-or-b "aca")
        "#,
            Ok(Some(
                Value::cons_list_unsanitized(vec![
                    Value::string_ascii_from_bytes(vec![0x61]).unwrap(),
                    Value::string_ascii_from_bytes(vec![0x62]).unwrap(),
                    Value::string_ascii_from_bytes(vec![0x61]).unwrap(),
                ])
                .unwrap(),
            )),
        )
    }

    #[test]
    fn test_map_simple_string_utf8() {
        crosscheck(
            r#"
(define-private (a-or-b (char (string-utf8 1))) (if (is-eq char u"a") u"a" u"b"))
(map a-or-b u"aca")
        "#,
            Ok(Some(
                Value::cons_list_unsanitized(vec![
                    Value::string_utf8_from_bytes(vec![0x61]).unwrap(),
                    Value::string_utf8_from_bytes(vec![0x62]).unwrap(),
                    Value::string_utf8_from_bytes(vec![0x61]).unwrap(),
                ])
                .unwrap(),
            )),
        )
    }

    #[test]
    fn test_map() {
        const MAP_FNS: &str = "
(define-private (addify-1 (a int))
  (+ a 1))

(define-private (addify-2 (a int) (b int))
  (+ a b 1))
";

        let a = &format!("{MAP_FNS} (map addify-1 (list 1 2 3))");
        crosscheck(a, evaluate("(list 2 3 4)"));

        let b = &format!("{MAP_FNS} (map addify-2 (list 1 2 3) (list 7 8))");
        crosscheck(b, evaluate("(list 9 11)"));
    }

    #[test]
    fn test_heterogeneus() {
        const MAP_HETERO: &str = "
(define-private (selectron (a bool) (b int) (c int))
  (if a b c))";

        let a = &format!(
            "{MAP_HETERO}
(map selectron
  (list true false false true)
  (list 1 2 3 4)
  (list 10 20 30))"
        );
        crosscheck(a, evaluate("(list 1 20 30)"));
    }

    #[test]
    fn test_builtin() {
        let a = "
(map +
  (list 1 2 3 4)
  (list 10 20 30))
";
        crosscheck(a, evaluate("(list 11 22 33)"))
    }

    #[test]
    fn map_and() {
        let a = "
(map and
  (list true true true)
  (list false true true)
  (list false false true))
";
        crosscheck(a, evaluate("(list false false true)"))
    }

    #[test]
    fn map_or() {
        let a = "
(map or
  (list true false true)
  (list false false true)
  (list false false false))
";
        crosscheck(a, evaluate("(list true false true)"));
    }

    #[test]
    fn map_divide() {
        let a = "(map / (list 1 4 9) (list 1 2 3))";
        crosscheck(a, evaluate("(list 1 2 3)"));
    }

    #[test]
    fn map_less_than_or_equal() {
        let a = "(map <= (list 1 3 3) (list 1 2 3))";
        crosscheck(a, evaluate("(list true false true)"));
    }

    #[test]
    fn map_less_than() {
        let a = "(map < (list 1 2 3) (list 1 3 3))";
        crosscheck(a, evaluate("(list false true false)"));
    }

    #[test]
    fn map_greater_than() {
        let a = "(map > (list 1 3 3) (list 1 2 3))";
        crosscheck(a, evaluate("(list false true false)"));
    }

    #[test]
    fn map_greater_than_or_equal() {
        let a = "(map >= (list 1 2 3) (list 1 3 3))";
        crosscheck(a, evaluate("(list true false true)"));
    }

    #[test]
    fn map_to_int() {
        let a = "(map to-int (list u1 u2 u3))";
        crosscheck(a, evaluate("(list 1 2 3)"));
    }

    #[test]
    fn map_log2() {
        let a = "(map log2 (list 1 2 3))";
        crosscheck(a, evaluate("(list 0 1 1)"));
    }

    #[test]
    fn map_mod() {
        let a = "(map mod (list 10 15 5) (list 1 2 3))";
        crosscheck(a, evaluate("(list 0 1 2)"));
    }

    #[test]
    fn map_mul() {
        let a = "(map * (list 1 2 3) (list 1 2 3))";
        crosscheck(a, evaluate("(list 1 4 9)"));
    }

    #[test]
    fn map_not() {
        let a = "(map not (list true false true false))";
        crosscheck(a, evaluate("(list false true false true)"));
    }

    #[test]
    fn map_pow() {
        let a = "(map pow (list 1 2 3) (list 1 2 3))";
        crosscheck(a, evaluate("(list 1 4 27)"));
    }

    #[test]
    fn map_sha512_256() {
        let a = "(map sha512/256 (list 1 2 3))";
        crosscheck(
            a,
            evaluate(
                "
        (list 
            0x515a7e92e7c60522db968d81ff70b80818fc17aeabbec36baf0dda2812e94a86
            0x541f557997791a762051eceb7c1069d9c903067d1d020bd38da294b10b0d680c
            0xe8107bb16a6b5f0cac737990336f93bc82bb678ba8a9cba86be3c3f818a34230
        )",
            ),
        );
    }

    #[test]
    fn map_sqrti() {
        let a = "(map sqrti (list 1 4 9))";
        crosscheck(a, evaluate("(list 1 2 3)"));
    }

    #[test]
    fn map_to_uint() {
        let a = "(map to-uint (list 1 2 3))";
        crosscheck(a, evaluate("(list u1 u2 u3)"));
    }

    #[test]
    fn map_xor() {
        let a = "(map xor (list 5 10 60) (list 1 2 -3))";
        crosscheck(a, evaluate("(list 4 8 -63)"));
    }

    #[test]
    fn map_keccak256() {
        let a = "(map keccak256 (list 1 2 3))";
        crosscheck(
            a,
            evaluate(
                "
        (list 
            0x97550c84a9e30d01461a29ac1c54c29e82c1925ee78b2ee1776d9e20c0183334
            0xf74616ab34b70062ff83d0f3459bee08066c0b32ed44ed6f4c52723036ee295c
            0x48dd032f5ebe0286a7aae330fe25a2fbe8e8288814e8f7ccb149f024611e71b1
        )",
            ),
        );
    }

    #[test]
    fn as_max_len_string_utf8() {
        crosscheck(
            r#"(as-max-len? u"hello" u16)"#,
            Ok(Some(
                Value::some(
                    Value::string_utf8_from_string_utf8_literal("hello".to_owned()).unwrap(),
                )
                .unwrap(),
            )),
        );
    }

    #[test]
    fn fold() {
        crosscheck(
            "
(define-private (sub (x int) (y int))
    (- x y))

(define-public (fold-sub)
    (ok (fold sub (list 1 2 3 4) 0)))

(fold-sub)
",
            evaluate("(ok 2)"),
        )
    }

    #[test]
    fn as_max_len_list() {
        crosscheck(
            r#"(as-max-len? (list 42 21) u2)"#,
            Ok(Some(
                Value::some(
                    Value::cons_list_unsanitized(vec![Value::Int(42), Value::Int(21)]).unwrap(),
                )
                .unwrap(),
            )),
        );
    }

    #[test]
    fn as_max_len_string_0() {
        crosscheck(
            r#"(as-max-len? "" u0)"#,
            Ok(Some(
                Value::some(Value::string_ascii_from_bytes(vec![]).unwrap()).unwrap(),
            )),
        );
    }

    #[test]
    fn as_max_len_list_0() {
        crosscheck(
            r#"(as-max-len? (list) u0)"#,
            Ok(Some(
                Value::some(Value::cons_list_unsanitized(vec![]).unwrap()).unwrap(),
            )),
        )
    }

    #[test]
    fn fold_bench() {
        crosscheck(
            "
(define-private (add-square (x int) (y int))
    (+ (* x x) y))

(define-public (fold-add-square (l (list 8192 int)) (init int))
    (ok (fold add-square l init)))

(fold-add-square (list 1 2 3 4) 3)
",
            evaluate("(ok 33)"),
        );
    }

    #[test]
    fn fold_sub() {
        crosscheck(
            "
(define-private (subalot (a int) (b int))
    (- b a))

(fold subalot (list 1 2 3 4 5) 399)
",
            Ok(Some(Value::Int(384))),
        );
    }

    #[test]
    fn map_sub() {
        crosscheck(
            "
(map - (list 1 2 3 4) (list 4 5 7 9) (list 41 51 71 9999))
",
            evaluate("(list -44 -54 -75 -10004)"),
        );
    }

    #[test]
    fn map_mul_regression() {
        crosscheck(
            "
(map * (list 0) (list 5) (list -34028236692093846346337460743176821146))
",
            evaluate("(list 0)"),
        );
    }

    #[test]
    fn map_unary() {
        crosscheck("(map - (list 10 20 30))", evaluate("(list -10 -20 -30)"));
    }

    #[test]
    fn map_repeated() {
        crosscheck(
            &"(map + (list 1 2 3) (list 1 2 3) (list 1 2 3))".repeat(700),
            Ok(Some(
                Value::cons_list_unsanitized(vec![Value::Int(3), Value::Int(6), Value::Int(9)])
                    .unwrap(),
            )),
        );
    }

    #[test]
    fn double_append() {
        let snippet = "(append (append (list 1) 2) 3)";

        let expected =
            Value::cons_list_unsanitized(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
                .unwrap();

        crosscheck(snippet, Ok(Some(expected)))
    }

    #[test]
    fn unit_fold_repsonses_full_type() {
        let snippet = "
(define-private (knus (a (response int int))
                      (b (response int int)))
  (match a
    a1 (match b
      b1 (err (+ a1 b1))
      b2 (ok  (- a1 b2)))
    a2 (match b
      b3 (ok  (+ a2 b3))
      b4 (err (- a2 b4)))))

(fold knus (list (ok 1)) (err 0))";

        crosscheck_compare_only(snippet);
    }

    #[test]
    fn unit_fold_repsonses_partial_type() {
        let snippet = "
(define-private (knus (a (response int int))
                      (b (response int int)))
  (match a
    a1 (match b
      b1 (err (+ a1 b1))
      b2 (ok  (- a1 b2)))
    a2 (match b
      b3 (ok  (+ a2 b3))
      b4 (err (- a2 b4)))))

(fold knus (list (err 1)) (err 0))";

        crosscheck_compare_only(snippet);
    }

    #[test]
    fn test_large_list() {
        let n = 50000 / 2 + 1;
        crosscheck_compare_only(&format!("(list {})", "9922 ".repeat(n)));
    }

    //
    // Module with tests that should only be executed
    // when running Clarity::V2 or Clarity::v3.
    //
    #[cfg(not(feature = "test-clarity-v1"))]
    #[cfg(test)]
    mod clarity_v2_v3 {
        use clarity::vm::errors::RuntimeErrorType;

        use super::*;

        #[test]
        fn test_map_mixed() {
            crosscheck(
                r#"
    (define-private (add-everything
        (a int)
        (b uint)
        (c (string-ascii 1))
        (d (string-utf8 1))
        (e (buff 1))
        )
        (+
            a
            (to-int b)
            (unwrap-panic (string-to-int? c))
            (unwrap-panic (string-to-int? d))
            (buff-to-int-be e)
        )
    )
    (map add-everything
        (list 1 2 3)
        (list u1 u2 u3)
        "123"
        u"123"
        0x010203
    )
            "#,
                Ok(Some(
                    Value::cons_list_unsanitized(vec![
                        Value::Int(5),
                        Value::Int(10),
                        Value::Int(15),
                    ])
                    .unwrap(),
                )),
            )
        }

        #[test]
        fn test_builtin_string() {
            let a = r#"
    (map >
      "ab"
      "ba"
    )"#;
            crosscheck(a, evaluate("(list false true)"));
        }

        #[test]
        fn map_large_result() {
            let n = 65535; // max legal `(list <size> uint)` size
            let buf = (0..n)
                .map(|i| format!("{:02x}", i % 256))
                .collect::<Vec<_>>()
                .join("");
            let snippet = format!(
                r#"
            (define-private (foo (a (buff 1))) (buff-to-uint-be a))
            (map foo 0x{buf})
            "#
            );

            crosscheck(
                &snippet,
                Ok(Some(
                    Value::cons_list_unsanitized((0..n).map(|c| Value::UInt(c % 256)).collect())
                        .unwrap(),
                )),
            );
        }

        #[test]
        fn slice_right_lt_left() {
            crosscheck("(slice? \"abc\" u1 u0)", evaluate("none"));
            crosscheck("(slice? \"abc\" u2 u1)", evaluate("none"));
        }

        #[test]
        fn slice_overflow() {
            crosscheck("(slice? \"abc\" u4 u5)", evaluate("none"));
        }

        #[test]
        fn slice() {
            crosscheck("(slice? \"abc\" u1 u2)", evaluate("(some \"b\")"));
        }

        #[test]
        fn slice_null() {
            crosscheck("(slice? \"abc\" u0 u0)", evaluate("(some \"\")"));
            crosscheck("(slice? \"abc\" u1 u1)", evaluate("(some \"\")"));
            crosscheck("(slice? \"abc\" u2 u2)", evaluate("(some \"\")"));
        }

        #[test]
        fn slice_full() {
            crosscheck("(slice? \"abc\" u0 u3)", evaluate("(some \"abc\")"));
        }

        #[test]
        fn replace_element_cannot_be_empty_buff() {
            let snippet = r#"(replace-at? 0x12345678 u0 0x)"#;

            crosscheck(
                snippet,
                Err(clarity::vm::errors::Error::Runtime(
                    RuntimeErrorType::BadTypeConstruction,
                    Some(Vec::new()),
                )),
            )
        }

        #[test]
        fn replace_element_cannot_be_empty_string_ascii() {
            let snippet = r#"(replace-at? "abcd" u0 "")"#;

            crosscheck(
                snippet,
                Err(clarity::vm::errors::Error::Runtime(
                    RuntimeErrorType::BadTypeConstruction,
                    Some(Vec::new()),
                )),
            )
        }

        #[test]
        fn replace_element_cannot_be_empty_string_utf8() {
            let snippet = r#"(replace-at? u"abcd" u0 u"")"#;

            crosscheck(
                snippet,
                Err(clarity::vm::errors::Error::Runtime(
                    RuntimeErrorType::BadTypeConstruction,
                    Some(Vec::new()),
                )),
            )
        }
        #[test]
        fn map_bit_and() {
            let a = "(map bit-and (list 1 2 3) (list 1 7 6) (list 1 15 15))";
            crosscheck(a, evaluate("(list 1 2 2)"));
        }

        #[test]
        fn map_bit_not() {
            let a = "(map bit-not (list 1 2 3))";
            crosscheck(a, evaluate("(list -2 -3 -4)"));
        }

        #[test]
        fn map_bit_or() {
            let a = "(map bit-or (list 1 2 3) (list 1 7 6) (list 1 15 15))";
            crosscheck(a, evaluate("(list 1 15 15)"));
        }

        #[test]
        fn map_bit_shift_left() {
            let a = "(map bit-shift-left (list u1 u2 u3) (list u2 u3 u4))";
            crosscheck(a, evaluate("(list u4 u16 u48)"));
        }

        #[test]
        fn map_bit_shift_right() {
            let a = "(map bit-shift-right (list u4 u16 u48) (list u2 u3 u4))";
            crosscheck(a, evaluate("(list u1 u2 u3)"));
        }

        #[test]
        fn map_bit_xor() {
            let a = "(map bit-xor (list 4 16 48) (list 2 3 4) (list 3 4 5))";
            crosscheck(a, evaluate("(list 5 23 49)"));
        }

        #[test]
        fn map_buff_to_int_be() {
            let a = "(map buff-to-int-be (list 0x010203 0x040506 0x070809))";
            crosscheck(a, evaluate("(list 66051 263430 460809)"));
        }

        #[test]
        fn map_buff_to_int_le() {
            let a = "(map buff-to-int-le (list 0x010203 0x040506 0x070809))";
            crosscheck(a, evaluate("(list 197121 394500 591879)"));
        }

        #[test]
        fn map_buff_to_uint_be() {
            let a = "(map buff-to-uint-be (list 0x010203 0x040506 0x070809))";
            crosscheck(a, evaluate("(list u66051 u263430 u460809)"));
        }

        #[test]
        fn map_buff_to_uint_le() {
            let a = "(map buff-to-uint-le (list 0x010203 0x040506 0x070809))";
            crosscheck(a, evaluate("(list u197121 u394500 u591879)"));
        }
        #[test]
        fn map_is_standard() {
            let a = "(map is-standard (list 'ST3X6QWWETNBZWGBK6DRGTR1KX50S74D3425Q1TPK 'SZ2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQ9H6DPR))";
            crosscheck(a, evaluate("(list true false)"));
        }

        #[test]
        fn map_principal_construct() {
            let snippet = "
(define-data-var index-local uint u0)
(define-data-var list-local (list 100 (buff 1)) (list ))
(define-public (test-principal-construct)
  (begin
    (var-set list-local (list 0x1a 0x1a))
    (ok (map test-principal-construct-inner (list 0xfa6bf38ed557fe417333710d6033e9419391a320 0x164247d6f2b425ac5771423ae6c80c754f7172b0)))
  )
)


(define-private (test-principal-construct-inner (pub-key-hash (buff 20)))
  (let
    (
      (index (var-get index-local))
    )
    (var-set index-local (+ u1 (var-get index-local)))
    (principal-construct? (unwrap-panic (element-at? (var-get list-local) index)) pub-key-hash)
  )
)
(test-principal-construct)";
            crosscheck(snippet, evaluate("
        (ok 
            (list 
                (ok 'ST3X6QWWETNBZWGBK6DRGTR1KX50S74D3425Q1TPK) (ok 'STB44HYPYAT2BB2QE513NSP81HTMYWBJP02HPGK6)
            )
        )"));
        }

        #[test]
        fn map_principal_destruct() {
            let a = "(map principal-destruct? (list 'ST3X6QWWETNBZWGBK6DRGTR1KX50S74D3425Q1TPK 'STB44HYPYAT2BB2QE513NSP81HTMYWBJP02HPGK6))";
            crosscheck(
                a,
                evaluate(
                    "
        (list 
            (ok 
                (tuple 
                    (hash-bytes 0xfa6bf38ed557fe417333710d6033e9419391a320) 
                    (name none) 
                    (version 0x1a)
                )
            ) 
            (ok 
                (tuple 
                    (hash-bytes 0x164247d6f2b425ac5771423ae6c80c754f7172b0) 
                    (name none) 
                    (version 0x1a)
                )
            )
        )",
                ),
            );
        }

        #[test]
        fn map_string_to_int() {
            let a = "(map string-to-int? (list \"1\" \"2\" \"3\"))";
            crosscheck(a, evaluate("(list (some 1) (some 2) (some 3))"));
        }

        #[test]
        fn map_string_to_uint() {
            let a = "(map string-to-uint? (list \"1\" \"2\" \"3\"))";
            crosscheck(a, evaluate("(list (some u1) (some u2) (some u3))"));
        }

        #[test]
        fn map_int_to_ascii() {
            let a = "(map int-to-ascii (list u1 u2 u3))";
            crosscheck(a, evaluate("(list \"1\" \"2\" \"3\")"));
        }

        #[test]
        fn map_int_to_utf8() {
            let a = "(map int-to-utf8 (list u1 u2 u3))";
            crosscheck(a, evaluate("(list u\"1\" u\"2\" u\"3\")"));
        }
    }
}
