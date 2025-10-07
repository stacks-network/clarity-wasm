use clarity::vm::types::signatures::CallableSubtype;
use clarity::vm::types::{SequenceSubtype, TupleTypeSignature, TypeSignature};
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ir::{BinaryOp, IfElse, InstrSeqType, Loop, UnaryOp};
use walrus::{InstrSeqBuilder, LocalId, ValType};

use super::{ComplexWord, Word};
use crate::check_args;
use crate::cost::WordCharge;
use crate::wasm_generator::{
    clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError, SequenceElementType, WasmGenerator,
};
use crate::wasm_utils::{check_argument_count, ArgumentCountCheck};

#[derive(Debug)]
pub struct IsEq;

impl Word for IsEq {
    fn name(&self) -> ClarityName {
        "is-eq".into()
    }
}

impl ComplexWord for IsEq {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let args_len = args.len();

        check_args!(generator, builder, 1, args_len, ArgumentCountCheck::AtLeast);

        self.charge(generator, builder, args_len as u32)?;

        // Since all argument should have compatible types, we unify them so that they all have the same representation.
        let unified_ty = args.iter().try_fold(TypeSignature::NoType, |ty, arg| {
            let arg_ty = generator.get_expr_type(arg).ok_or_else(|| {
                GeneratorError::TypeError("Is-eq argument should be typed".to_owned())
            })?;
            TypeSignature::least_supertype(&generator.contract_analysis.epoch, &ty, arg_ty)
                .map_err(|e| GeneratorError::TypeError(format!("Incompatible types in is-eq: {e}")))
        })?;
        for a in args {
            generator.set_expr_type(a, unified_ty.clone())?;
        }

        // Traverse the first operand pushing it onto the stack
        let first_op = args.get_expr(0)?;
        generator.traverse_expr(builder, first_op)?;
        let ty = generator
            .get_expr_type(first_op)
            .ok_or_else(|| {
                GeneratorError::TypeError("is-eq value expression must be typed".to_owned())
            })?
            .clone();

        // No need to go further if there is only one argument
        if args.len() == 1 {
            drop_value(builder, &ty);
            builder.i32_const(1); // TRUE
            return Ok(());
        }

        // Save the first_op to a local to be further used.
        // This allows to use the first_op value without
        // traversing again the expression.
        let wasm_types = clar2wasm_ty(&ty);
        let val_locals: Vec<_> = wasm_types
            .iter()
            .map(|local_ty| generator.module.locals.add(*local_ty))
            .collect();
        assign_first_operand_to_locals(builder, &ty, &val_locals)?;

        // initialize (reusable) locals for the other operands
        let nth_locals: Vec<_> = wasm_types
            .iter()
            .map(|local_ty| generator.module.locals.add(*local_ty))
            .collect();

        // Initialize boolean result accumulator to TRUE
        builder.i32_const(1);

        // Loop through remainder operands, if the case.
        for operand in args.iter().skip(1) {
            // push the new operand on the stack
            generator.traverse_expr(builder, operand)?;

            // insert the new operand into locals
            let operand_ty = generator
                .get_expr_type(operand)
                .ok_or_else(|| {
                    GeneratorError::TypeError("is-eq value expression must be typed".to_owned())
                })?
                .clone();
            assign_to_locals(builder, &ty, &operand_ty, &nth_locals)?;

            // check equality
            wasm_equal(&ty, generator, builder, &val_locals, &nth_locals)?;

            // Do an "and" operation with the result from the previous function call.
            builder.binop(BinaryOp::I32And);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum IndexOf {
    Original,
    Alias,
}

impl Word for IndexOf {
    fn name(&self) -> ClarityName {
        match self {
            IndexOf::Original => "index-of".into(),
            IndexOf::Alias => "index-of?".into(),
        }
    }
}

impl ComplexWord for IndexOf {
    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        // Traverse the sequence, leaving its offset and size on the stack.
        let seq = args.get_expr(0)?;
        let elem_expr = args.get_expr(1)?;
        // workaround to fix types in the case of elements that are themself Sequences
        if let TypeSignature::SequenceType(SequenceSubtype::ListType(ltd)) = generator
            .get_expr_type(seq)
            .ok_or(GeneratorError::TypeError(
                "index_of element must be typed".to_owned(),
            ))?
        {
            generator.set_expr_type(elem_expr, ltd.get_list_item_type().clone())?;
        }

        // Traverse the sequence, leaving its offset and size on the stack.
        generator.traverse_expr(builder, seq)?;
        // STACK: [offset, size]

        // Get type of the Sequence element.
        let elem_ty = generator.get_sequence_element_type(seq)?;

        // Locals declaration.
        let seq_size = generator.module.locals.add(ValType::I32);
        let offset = generator.module.locals.add(ValType::I32);
        let end_offset = generator.module.locals.add(ValType::I32);

        builder
            .local_set(seq_size)
            // STACK: [offset]
            .local_tee(offset)
            // STACK: [offset]
            .local_get(seq_size)
            // STACK: [offset, size]
            .binop(BinaryOp::I32Add)
            // STACK: [add_result]
            .local_set(end_offset);
        // STACK: []

        self.charge(generator, builder, seq_size)?;

        builder.local_get(seq_size).unop(UnaryOp::I32Eqz);
        // STACK: [size]

        let ty = InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &[ValType::I32, ValType::I64, ValType::I64],
        );

        let if_id = {
            let mut if_case = builder.dangling_instr_seq(ty);
            if_case.i32_const(0).i64_const(0).i64_const(0);
            if_case.id()
        };

        let else_id = {
            let else_case = &mut builder.dangling_instr_seq(ty);
            let item = args.get_expr(1)?;
            generator.traverse_expr(else_case, item)?;
            // STACK: [item]

            // Get the type of the item expression
            let item_ty = generator
                .get_expr_type(item)
                .ok_or_else(|| {
                    GeneratorError::TypeError("index_of item expression must be typed".to_owned())
                })?
                .clone();

            // Store the item into a local.
            let item_locals = generator.save_to_locals(else_case, &item_ty, true);
            // STACK: []

            // Create and store an index into a local.
            let index = generator.module.locals.add(ValType::I64);
            else_case.i64_const(0);
            // STACK: [0]
            else_case.local_set(index);
            // STACK: []

            // Loop through the sequence.
            let loop_body_ty = InstrSeqType::new(
                &mut generator.module.types,
                &[],
                &[ValType::I32, ValType::I64, ValType::I64],
            );

            let loop_body = &mut else_case.dangling_instr_seq(loop_body_ty);
            let loop_body_id = {
                // Loop label.
                let loop_id = loop_body.id();

                // Load an element from the sequence, at offset position,
                // and push it onto the top of the stack.
                // Also store the current sequence element into a local.
                let (elem_size, elem_locals) = match &elem_ty {
                    SequenceElementType::Other(elem_ty) => {
                        (
                            generator.read_from_memory(loop_body, offset, 0, elem_ty)?,
                            // STACK: [element]
                            generator.save_to_locals(loop_body, elem_ty, true),
                            // STACK: []
                        )
                    }
                    SequenceElementType::Byte => {
                        // The element type is a byte, so we can just push the
                        // offset and size = 1 to the stack.
                        let size = 1;
                        loop_body.local_get(offset).i32_const(size);
                        // STACK: [offset, size]

                        (size, generator.save_to_locals(loop_body, &item_ty, true))
                        // STACK: []
                    }
                    SequenceElementType::UnicodeScalar => {
                        // The element type is a unicode scalar, so we can just push the
                        // offset and size = 4 to the stack.
                        let size = 4;
                        loop_body.local_get(offset).i32_const(size);
                        // STACK: [offset, size]

                        (size, generator.save_to_locals(loop_body, &item_ty, true))
                        // STACK: []
                    }
                };

                // Check item and element equality.
                // And push the result of the comparison onto the top of the stack.
                wasm_equal(&item_ty, generator, loop_body, &item_locals, &elem_locals)?;
                // STACK: [wasm_equal_result]

                loop_body.if_else(
                    InstrSeqType::new(
                        &mut generator.module.types,
                        &[],
                        &[ValType::I32, ValType::I64, ValType::I64],
                    ),
                    |then| {
                        then.i32_const(1).local_get(index).i64_const(0);
                        // STACK: [1, index_lo, index_hi]
                    },
                    |else_| {
                        // Increment the sequence offset by the size of the element
                        // and push it to the stack.
                        // Also push the offset limit onto the top of the stack.
                        else_
                            .local_get(offset)
                            .i32_const(elem_size)
                            .binop(BinaryOp::I32Add)
                            .local_tee(offset)
                            .local_get(end_offset);
                        // STACK: [offset, end_offset]

                        else_.binop(BinaryOp::I32GeU).if_else(
                            InstrSeqType::new(
                                &mut generator.module.types,
                                &[],
                                &[ValType::I32, ValType::I64, ValType::I64],
                            ),
                            |then| {
                                // Reached the end of the sequence
                                // and not found the element.
                                then.i32_const(0).local_get(index).i64_const(0);
                                // STACK: [0, index_lo, index_hi]
                            },
                            |else_| {
                                // Increment index by 1
                                // and continue loop.
                                else_
                                    .local_get(index)
                                    .i64_const(1)
                                    .binop(BinaryOp::I64Add)
                                    .local_set(index)
                                    .br(loop_id);
                            },
                        );
                    },
                );
                loop_body.id()
            };

            else_case.instr(Loop { seq: loop_body_id });

            else_case.id()
        };

        builder.instr(IfElse {
            consequent: if_id,
            alternative: else_id,
        });

        Ok(())
    }
}

fn assign_to_locals(
    builder: &mut walrus::InstrSeqBuilder,
    original_ty: &TypeSignature,
    current_ty: &TypeSignature,
    locals: &[LocalId],
) -> Result<(), GeneratorError> {
    // WE HAVE TO GO THROUGH LOCALS IN REVERSE ORDER!!!
    match (original_ty, current_ty) {
        // Any NoType isn't worth assigning to a local, and can just be dropped
        (TypeSignature::NoType, _) | (_, TypeSignature::NoType) => {
            drop_value(builder, current_ty);
        }
        (TypeSignature::OptionalType(t), TypeSignature::OptionalType(s)) => {
            let (variant_local, inner_locals) = locals.split_first().ok_or_else(|| {
                GeneratorError::InternalError("missing locals for optional variant".to_string())
            })?;
            assign_to_locals(builder, t, s, inner_locals)?;
            builder.local_set(*variant_local);
        }
        (TypeSignature::ResponseType(t), TypeSignature::ResponseType(s)) => {
            let (variant_local, inner_locals) = locals.split_first().ok_or_else(|| {
                GeneratorError::InternalError("missing locals for response variant".to_string())
            })?;
            let first_ok_size = clar2wasm_ty(&t.0).len();
            let (ok_locals, err_locals) = inner_locals.split_at(first_ok_size);
            assign_to_locals(builder, &t.1, &s.1, err_locals)?;
            assign_to_locals(builder, &t.0, &s.0, ok_locals)?;
            builder.local_set(*variant_local);
        }
        (TypeSignature::TupleType(t), TypeSignature::TupleType(s)) => {
            let mut remaining_locals = locals;
            for (tt, ss) in t
                .get_type_map()
                .values()
                .rev()
                .zip(s.get_type_map().values().rev())
            {
                let tt_size = clar2wasm_ty(tt).len();
                let (rest, cur_locals) =
                    remaining_locals.split_at(remaining_locals.len() - tt_size);
                remaining_locals = rest;
                assign_to_locals(builder, tt, ss, cur_locals)?;
            }
        }
        // All the other types aren't influenced by inner NoType and can just be assigned automatically
        _ => {
            for i in (0..clar2wasm_ty(original_ty).len()).rev() {
                builder.local_set(*locals.get(i).ok_or_else(|| {
                    GeneratorError::InternalError("not enough locals for simple type".to_string())
                })?);
            }
        }
    }
    Ok(())
}

fn assign_first_operand_to_locals(
    builder: &mut walrus::InstrSeqBuilder,
    ty: &TypeSignature,
    locals: &[LocalId],
) -> Result<(), GeneratorError> {
    assign_to_locals(builder, ty, ty, locals)
}

fn wasm_equal(
    ty: &TypeSignature,
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
) -> Result<(), GeneratorError> {
    match ty {
        // we should never compare NoType
        TypeSignature::NoType => {
            builder.unreachable();
            Ok(())
        }
        TypeSignature::BoolType => {
            builder
                .local_get(first_op[0])
                .local_get(nth_op[0])
                .binop(BinaryOp::I32Eq);
            Ok(())
        }
        // is-eq-int function can be reused to both int and uint types.
        TypeSignature::IntType | TypeSignature::UIntType => {
            wasm_equal_int128(generator, builder, first_op, nth_op)
        }
        // is-eq-bytes function can be used for types with (offset, length)
        TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
        | TypeSignature::SequenceType(SequenceSubtype::StringType(_)) => {
            wasm_equal_bytes(generator, builder, first_op, nth_op)
        }
        TypeSignature::PrincipalType
        | TypeSignature::CallableType(CallableSubtype::Principal(_)) => {
            wasm_equal_bytes(generator, builder, first_op, nth_op)
        }
        TypeSignature::OptionalType(some_ty) => {
            wasm_equal_optional(generator, builder, first_op, nth_op, some_ty)
        }

        TypeSignature::ResponseType(ok_err_ty) => wasm_equal_response(
            generator,
            builder,
            first_op,
            nth_op,
            &ok_err_ty.0,
            &ok_err_ty.1,
        ),
        TypeSignature::TupleType(tuple_ty) => {
            wasm_equal_tuple(generator, builder, first_op, nth_op, tuple_ty)
        }

        TypeSignature::SequenceType(SequenceSubtype::ListType(list_ty)) => wasm_equal_list(
            generator,
            builder,
            first_op,
            nth_op,
            list_ty.get_list_item_type(),
        ),

        _ => Err(GeneratorError::NotImplemented),
    }
}

fn wasm_equal_int128(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
) -> Result<(), GeneratorError> {
    // Get first operand from the local and put it onto stack.
    for val in first_op {
        builder.local_get(*val);
    }

    // Get second operand from the local and put it onto stack.
    for val in nth_op {
        builder.local_get(*val);
    }

    // Call the function with the operands on the stack.
    let func = generator.func_by_name("stdlib.is-eq-int");
    builder.call(func);

    Ok(())
}

fn wasm_equal_bytes(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
) -> Result<(), GeneratorError> {
    // Get first operand from the local and put it onto stack.
    for val in first_op {
        builder.local_get(*val);
    }

    // Get second operand from the local and put it onto stack.
    for val in nth_op {
        builder.local_get(*val);
    }

    // Call the function with the operands on the stack.
    let func = generator.func_by_name("stdlib.is-eq-bytes");
    builder.call(func);

    Ok(())
}

fn wasm_equal_optional(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
    some_ty: &TypeSignature,
) -> Result<(), GeneratorError> {
    let Some((first_variant, first_inner)) = first_op.split_first() else {
        return Err(GeneratorError::InternalError(
            "Optional operand should have at least one argument".into(),
        ));
    };
    let Some((nth_variant, nth_inner)) = nth_op.split_first() else {
        return Err(GeneratorError::InternalError(
            "Optional operand should have at least one argument".into(),
        ));
    };

    // check if we have (some x, some x) or (none, none)
    builder
        .local_get(*first_variant)
        .local_get(*nth_variant)
        .binop(BinaryOp::I32Eq);

    // if both operands are identical,
    // [then]: we check if we have a `none` (automatic true) or if the `some` inner_type are equal
    // [else]: we push "false" on the stack
    let then_id = {
        let mut then = builder.dangling_instr_seq(ValType::I32);

        let none_case_id = {
            let mut none_ = then.dangling_instr_seq(ValType::I32);
            none_.i32_const(1);
            none_.id()
        };

        let some_case_id = {
            let mut some_ = then.dangling_instr_seq(ValType::I32);
            wasm_equal(some_ty, generator, &mut some_, first_inner, nth_inner)?;
            some_.id()
        };

        // put those in an if statement (true if `some`, false if `none`)
        then.local_get(*first_variant).instr(IfElse {
            consequent: some_case_id,
            alternative: none_case_id,
        });

        then.id()
    };

    let else_id = {
        let mut else_ = builder.dangling_instr_seq(ValType::I32);
        else_.i32_const(0);
        else_.id()
    };

    builder.instr(IfElse {
        consequent: then_id,
        alternative: else_id,
    });

    Ok(())
}

fn wasm_equal_response(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
    ok_ty: &TypeSignature,
    err_ty: &TypeSignature,
) -> Result<(), GeneratorError> {
    let Some((first_variant, first_ok, first_err)) =
        first_op.split_first().map(|(variant, rest)| {
            let split_ok_err_idx = clar2wasm_ty(ok_ty).len();
            let (ok, err) = rest.split_at(split_ok_err_idx);
            (variant, ok, err)
        })
    else {
        return Err(GeneratorError::InternalError(
            "Response operand should have at least one argument".into(),
        ));
    };
    let Some((nth_variant, nth_ok, nth_err)) = nth_op.split_first().map(|(variant, rest)| {
        let split_ok_err_idx = clar2wasm_ty(ok_ty).len();
        let (ok, err) = rest.split_at(split_ok_err_idx);
        (variant, ok, err)
    }) else {
        return Err(GeneratorError::InternalError(
            "Response operand should have at least one argument".into(),
        ));
    };

    // We will have a three branch if:
    // [ok] is the (ok, ok) case, we have to compare if both ok values are identical
    // [err] is the (err, err) case, we have to compare if both err values are identical
    // [else] is the (ok, err) or (err, ok) case, it is directly false

    let ok_id = {
        let mut ok_case = builder.dangling_instr_seq(ValType::I32);
        wasm_equal(ok_ty, generator, &mut ok_case, first_ok, nth_ok)?;
        ok_case.id()
    };

    let err_id = {
        let mut err_case = builder.dangling_instr_seq(ValType::I32);
        wasm_equal(err_ty, generator, &mut err_case, first_err, nth_err)?;
        err_case.id()
    };

    let else_id = {
        let mut else_ = builder.dangling_instr_seq(ValType::I32);
        else_.i32_const(0);
        else_.id()
    };

    // inner if is checking if both are err (consequent) or ok (alternative)
    let inner_if_id = {
        let mut inner_if = builder.dangling_instr_seq(ValType::I32);
        inner_if.local_get(*first_variant).instr(IfElse {
            consequent: ok_id,
            alternative: err_id,
        });
        inner_if.id()
    };

    // outer if checks if both variants are identical (consequent) or not (alternative)
    builder
        .local_get(*first_variant)
        .local_get(*nth_variant)
        .binop(BinaryOp::I32Eq)
        .instr(IfElse {
            consequent: inner_if_id,
            alternative: else_id,
        });

    Ok(())
}

fn wasm_equal_tuple(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
    tuple_ty: &TupleTypeSignature,
) -> Result<(), GeneratorError> {
    // we'll compare tuple lazily field by field, so that
    // `(is-eq {x: a1, y: a2, z: a3} {x: b1, y: b2, z: b3})` becomes
    // ```
    // if (a1 == b1)
    //   then if (a2 == b2)
    //     then (a3 == b3)
    //     else false
    //   else false
    // ```
    // we have to build the if sequence bottom-up

    let field_types = tuple_ty.get_type_map();

    // this is the number of elements in the tuple. Always >= 1 due to Clarity constraints.
    let mut depth = field_types.len();

    // this is an iterator in reverse order (for bottom-up sequence) of
    // `(ty, range)`, where `ty` is the type of the current tuple element and `range` is
    // the range index of this element in the list of locals
    let mut wasm_ranges = field_types.values().rev().scan(
        field_types.values().map(|ty| clar2wasm_ty(ty).len()).sum(),
        |i, ty| {
            (*i != 0).then(|| {
                let wasm_ty = clar2wasm_ty(ty);
                let old = *i;
                *i -= wasm_ty.len();
                (ty, *i..old)
            })
        },
    );

    // if this is a 1-tuple, we can just check for equality of element
    if depth == 1 {
        let (ty, range) = wasm_ranges.next().ok_or_else(|| {
            GeneratorError::InternalError("Expected first tuple type for comparison".to_owned())
        })?;

        return wasm_equal(
            ty,
            generator,
            builder,
            &first_op[range.clone()],
            &nth_op[range],
        );
    }

    // bottom equality statement
    let mut instr_id = {
        let mut instr = builder.dangling_instr_seq(ValType::I32);
        let (ty, range) = wasm_ranges.next().ok_or_else(|| {
            GeneratorError::InternalError("Expected first tuple type for comparison".to_owned())
        })?;

        wasm_equal(
            ty,
            generator,
            &mut instr,
            &first_op[range.clone()],
            &nth_op[range],
        )?;

        instr.id()
    };
    depth -= 1;

    // intermediary if-else statements
    while depth > 1 {
        let (ty, range) = wasm_ranges.next().ok_or_else(|| {
            GeneratorError::InternalError("Expected first tuple type for comparison".to_owned())
        })?;

        let else_id = {
            let mut else_ = builder.dangling_instr_seq(ValType::I32);
            else_.i32_const(0);
            else_.id()
        };

        instr_id = {
            let mut if_else = builder.dangling_instr_seq(ValType::I32);

            wasm_equal(
                ty,
                generator,
                &mut if_else,
                &first_op[range.clone()],
                &nth_op[range],
            )?;

            if_else.instr(IfElse {
                consequent: instr_id,
                alternative: else_id,
            });

            if_else.id()
        };

        depth -= 1;
    }

    // top if-else statement
    let (ty, range) = wasm_ranges.next().ok_or_else(|| {
        GeneratorError::InternalError("Expected first tuple type for comparison".to_owned())
    })?;
    let top_else_id = {
        let mut else_ = builder.dangling_instr_seq(ValType::I32);
        else_.i32_const(0);
        else_.id()
    };

    wasm_equal(
        ty,
        generator,
        builder,
        &first_op[range.clone()],
        &nth_op[range],
    )?;

    builder.instr(IfElse {
        consequent: instr_id,
        alternative: top_else_id,
    });

    Ok(())
}

fn wasm_equal_list(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    first_op: &[LocalId],
    nth_op: &[LocalId],
    list_ty: &TypeSignature,
) -> Result<(), GeneratorError> {
    let [offset_a, len_a] = first_op else {
        return Err(GeneratorError::InternalError(
            "List type should have two i32 locals: offset and length".to_string(),
        ));
    };
    let [offset_b, len_b] = nth_op else {
        return Err(GeneratorError::InternalError(
            "List type should have two i32 locals: offset and length".to_string(),
        ));
    };

    let first_wasm_types = clar2wasm_ty(list_ty);
    let first_locals: Vec<_> = first_wasm_types
        .iter()
        .map(|local_ty| generator.module.locals.add(*local_ty))
        .collect();

    let nth_locals: Vec<_> = first_wasm_types
        .iter()
        .map(|local_ty| generator.module.locals.add(*local_ty))
        .collect();

    // need offset_delta for both types = clar2wasm_ty(list_ty).len()
    // those are the result of `generator.read_from_memory`, which is computed
    // in a block later, hence the declaration here.
    let offset_delta_a;
    let offset_delta_b;

    // if len_a != len_b { false } else if len_a == 0 { true } else LOOP

    let not_equal_sizes = {
        let mut instr = builder.dangling_instr_seq(ValType::I32);
        instr.i32_const(0);
        instr.id()
    };

    let empty_lists = {
        let mut instr = builder.dangling_instr_seq(ValType::I32);
        instr.i32_const(1);
        instr.id()
    };

    let comparison_loop = {
        let mut instr = builder.dangling_instr_seq(ValType::I32);

        let loop_id = {
            let mut loop_ = instr.dangling_instr_seq(None);
            let loop_id = loop_.id();

            // read an element from first list and assign it to locals
            offset_delta_a = generator.read_from_memory(&mut loop_, *offset_a, 0, list_ty)?;
            assign_first_operand_to_locals(&mut loop_, list_ty, &first_locals)?;

            // same for nth list
            offset_delta_b = generator.read_from_memory(&mut loop_, *offset_b, 0, list_ty)?;
            assign_to_locals(&mut loop_, list_ty, list_ty, &nth_locals)?;

            // compare both elements
            wasm_equal(list_ty, generator, &mut loop_, &first_locals, &nth_locals)?;

            // if there is equality, we update the variables and we loop
            loop_.if_else(
                None,
                |then| {
                    // increment the lists offsets
                    then.local_get(*offset_a)
                        .i32_const(offset_delta_a)
                        .binop(BinaryOp::I32Add)
                        .local_set(*offset_a);
                    then.local_get(*offset_b)
                        .i32_const(offset_delta_b)
                        .binop(BinaryOp::I32Add)
                        .local_set(*offset_b);

                    // loop while we still have elements
                    then.local_get(*len_b)
                        .i32_const(offset_delta_b)
                        .binop(BinaryOp::I32Sub)
                        .local_tee(*len_b)
                        .br_if(loop_id);
                },
                |_| {},
            );

            loop_id
        };

        // Now that we have our comparison loop, we add it to the instructions.
        // After it, we just have to check if the counter `len_b` is at 0, indicating
        // we looped through all elements and everything is equal
        // In case we have 3 or more operands for `is-eq`, we also should make sure that
        // *offset_a* is reset at the end of the loop. We accomplish that by putting its original
        // value on the stack before the loop and setting it back after the loop.
        instr
            .local_get(*offset_a)
            .instr(Loop { seq: loop_id })
            .local_set(*offset_a)
            .local_get(*len_b)
            .unop(UnaryOp::I32Eqz);
        instr.id()
    };

    // if-else when sizes are identical
    let equal_size_id = {
        let mut instr = builder.dangling_instr_seq(ValType::I32);
        // consequent when size is 0; alternative when size > 0
        instr.local_get(*len_b).unop(UnaryOp::I32Eqz).instr(IfElse {
            consequent: empty_lists,
            alternative: comparison_loop,
        });
        instr.id()
    };

    // if-else sizes are equal or not?
    builder
        .local_get(*len_a)
        .local_get(*len_b)
        .binop(BinaryOp::I32Eq)
        // consequent when same sizes, alternative for different sizes
        .instr(IfElse {
            consequent: equal_size_id,
            alternative: not_equal_sizes,
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::{ListData, ListTypeData, SequenceData};
    use clarity::vm::Value;

    use crate::tools::{crosscheck, evaluate, TestEnvironment};

    #[test]
    fn is_eq_less_than_one_arg() {
        let result = evaluate("(is-eq)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting >= 1 arguments, got 0"));
    }

    #[test]
    fn index_of_list_less_than_two_args() {
        let result = evaluate("(index-of (list 1 2 3))");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn index_of_list_more_than_two_args() {
        let result = evaluate("(index-of (list 1 2 3) 1 2)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn index_of_list_not_present() {
        crosscheck("(index-of (list 1 2 3 4 5 6 7) 9)", Ok(Some(Value::none())));
    }

    #[test]
    fn index_of_list_first() {
        crosscheck(
            "(index-of (list 1 2 3 4) 1)",
            Ok(Some(Value::some(Value::UInt(0)).unwrap())),
        );
    }

    #[test]
    fn index_of_list() {
        crosscheck(
            "(index-of (list 1 2 3 4 5 6 7) 3)",
            Ok(Some(Value::some(Value::UInt(2)).unwrap())),
        );
    }

    #[test]
    fn index_of_list_last() {
        crosscheck(
            "(index-of (list 1 2 3 4 5 6 7) 7)",
            Ok(Some(Value::some(Value::UInt(6)).unwrap())),
        );
    }

    #[test]
    fn index_of_list_called_by_v1_alias() {
        crosscheck(
            "(index-of (list 1 2 3 4 5 6 7) 100)",
            Ok(Some(Value::none())),
        );
    }

    #[test]
    fn index_of_list_of_lists() {
        crosscheck("(index-of (list (list 1 2) (list 2 3 4) (list 1 2 3 4 5) (list 1 2 3 4)) (list 1 2 3 4))",
            Ok(Some(Value::some(Value::UInt(3)).unwrap()))
        );
    }

    #[test]
    fn index_of_list_zero_len() {
        let mut env = TestEnvironment::default();
        let val = env.init_contract_with_snippet(
            "index_of",
            r#"
(define-private (find-it? (needle int) (haystack (list 10 int)))
  (index-of haystack needle))
(find-it? 6 (list))
"#,
        );

        assert_eq!(val.unwrap(), Some(Value::none()));
    }

    #[test]
    fn index_of_list_check_stack() {
        let mut env = TestEnvironment::default();
        let val = env.evaluate(
            r#"
(define-private (find-it? (needle int) (haystack (list 10 int)))
  (is-eq (index-of haystack needle) none))
(asserts! (find-it? 6 (list 1 2 3)) (err u1))
(list 4 5 6)
"#,
        );

        assert_eq!(
            val.unwrap(),
            Some(Value::Sequence(SequenceData::List(ListData {
                data: vec![Value::Int(4), Value::Int(5), Value::Int(6)],
                type_signature: ListTypeData::new_list(
                    clarity::vm::types::TypeSignature::IntType,
                    3
                )
                .unwrap()
            })))
        );
    }

    #[test]
    fn index_of_ascii() {
        crosscheck(
            "(index-of \"Stacks\" \"a\")",
            Ok(Some(Value::some(Value::UInt(2)).unwrap())),
        );
    }

    #[test]
    fn index_of_ascii_empty() {
        crosscheck("(index-of \"\" \"\")", Ok(Some(Value::none())));
    }

    #[test]
    fn index_of_ascii_empty_input() {
        crosscheck("(index-of \"\" \"a\")", Ok(Some(Value::none())));
    }

    #[test]
    fn index_of_ascii_empty_char() {
        crosscheck("(index-of \"Stacks\" \"\")", Ok(Some(Value::none())));
    }

    #[test]
    fn index_of_ascii_first_elem() {
        crosscheck(
            "(index-of \"Stacks\" \"S\")",
            Ok(Some(Value::some(Value::UInt(0)).unwrap())),
        );
    }

    #[test]
    fn index_of_ascii_last_elem() {
        crosscheck(
            "(index-of \"Stacks\" \"s\")",
            Ok(Some(Value::some(Value::UInt(5)).unwrap())),
        );
    }

    #[test]
    fn index_of_utf8() {
        crosscheck(
            "(index-of u\"Stacks\" u\"a\")",
            Ok(Some(Value::some(Value::UInt(2)).unwrap())),
        );
    }

    #[test]
    fn index_of_utf8_b() {
        crosscheck(
            "(index-of u\"St\\u{1F98A}cks\" u\"\\u{1F98A}\")",
            Ok(Some(Value::some(Value::UInt(2)).unwrap())),
        );
    }

    #[test]
    fn index_of_utf8_first_elem() {
        crosscheck(
            "(index-of u\"Stacks\\u{1F98A}\" u\"S\")",
            Ok(Some(Value::some(Value::UInt(0)).unwrap())),
        );
    }

    #[test]
    fn index_of_utf8_last_elem() {
        crosscheck(
            "(index-of u\"Stacks\\u{1F98A}\" u\"\\u{1F98A}\")",
            Ok(Some(Value::some(Value::UInt(6)).unwrap())),
        );
    }

    #[test]
    fn index_of_utf8_zero_len() {
        crosscheck("(index-of u\"Stacks\" u\"\")", Ok(Some(Value::none())));
    }

    #[test]
    fn index_of_buff_last_byte() {
        crosscheck(
            "(index-of 0xfb01 0x01)",
            Ok(Some(Value::some(Value::UInt(1)).unwrap())),
        );
    }

    #[test]
    fn index_of_buff_first_byte() {
        crosscheck(
            "(index-of 0xfb01 0xfb)",
            Ok(Some(Value::some(Value::UInt(0)).unwrap())),
        );
    }

    #[test]
    fn index_of_buff() {
        crosscheck(
            "(index-of 0xeeaadd 0xaa)",
            Ok(Some(Value::some(Value::UInt(1)).unwrap())),
        );
    }

    #[test]
    fn index_of_buff_not_present() {
        crosscheck("(index-of 0xeeaadd 0xcc)", Ok(Some(Value::none())));
    }

    #[test]
    fn index_of_first_optional_complex_type() {
        crosscheck(
            "(index-of (list (some 42) none none none (some 15)) (some 42))",
            Ok(Some(Value::some(Value::UInt(0)).unwrap())),
        );
    }

    #[test]
    fn index_of_last_optional_complex_type() {
        crosscheck(
            "(index-of (list (some 42) (some 3) (some 6) (some 15) none) none)",
            Ok(Some(Value::some(Value::UInt(4)).unwrap())),
        );
    }

    #[test]
    fn index_of_optional_complex_type() {
        crosscheck(
            "(index-of (list (some 1) none) none)",
            Ok(Some(Value::some(Value::UInt(1)).unwrap())),
        );
    }

    #[test]
    fn index_of_tuple_complex_type() {
        crosscheck("(index-of (list (tuple (id 42) (name \"Clarity\")) (tuple (id 133) (name \"Wasm\"))) (tuple (id 42) (name \"Wasm\")))",
            Ok(Some(Value::none()))
        );
    }

    #[test]
    fn is_eq_equal_buffers_with_different_max_len() {
        let snippet = "
        (define-data-var a (buff 2) 0x00)
        (define-data-var b (buff 3) 0x00)
        (is-eq (var-get a) (var-get b))";
        crosscheck(snippet, Ok(Some(clarity::vm::Value::Bool(true))));
    }

    #[test]
    fn is_eq_equal_ascii_strings_with_different_max_len() {
        let snippet = "
        (define-data-var a (string-ascii 3) \"lol\")
        (define-data-var b (string-ascii 4) \"lol\")
        (is-eq (var-get a) (var-get b))";
        crosscheck(snippet, Ok(Some(clarity::vm::Value::Bool(true))));
    }

    #[test]
    fn is_eq_equal_utf8_strings_with_different_max_len() {
        let snippet = r#"
        (define-data-var a (string-utf8 22) u"lol")
        (define-data-var b (string-utf8 21) u"lol")
        (is-eq (var-get a) (var-get b))"#;
        crosscheck(snippet, Ok(Some(clarity::vm::Value::Bool(true))));
    }

    #[test]
    fn is_eq_equal_lists_with_different_max_len() {
        let snippet = "
        (define-data-var a (list 3 int) (list 1 2 3))
        (define-data-var b (list 4 int) (list 1 2 3))
        (is-eq (var-get a) (var-get b))";
        crosscheck(snippet, Ok(Some(clarity::vm::Value::Bool(true))));
    }

    #[test]
    fn index_of_complex_type() {
        crosscheck(
            "(index-of (list (list (ok 2) (err 5)) (list (ok 42)) (list (err 7))) (list (err 7)))",
            Ok(Some(Value::some(Value::UInt(2)).unwrap())),
        );
    }

    //
    // Module with tests that should only be executed
    // when running Clarity::V2 or Clarity::v3.
    //
    #[cfg(not(feature = "test-clarity-v1"))]
    #[cfg(test)]
    mod clarity_v2_v3 {
        use super::*;
        use crate::tools::crosscheck;

        #[test]
        fn index_of_alias_list_zero_len() {
            let mut env = TestEnvironment::default();
            let val = env.init_contract_with_snippet(
                "index_of",
                r#"
    (define-private (find-it? (needle int) (haystack (list 10 int)))
      (index-of? haystack needle))
    (find-it? 6 (list))
    "#,
            );

            assert_eq!(val.unwrap(), Some(Value::none()));
        }

        #[test]
        fn index_of_alias_first_optional_complex_type() {
            crosscheck(
                "(index-of? (list (some 42) none none none (some 15)) (some 42))",
                Ok(Some(Value::some(Value::UInt(0)).unwrap())),
            );
        }
    }

    #[test]
    fn is_eq_with_different_operands_types() {
        let snippet = "(is-eq (err false) (if true (ok u1) (err true)))";

        crosscheck(snippet, Ok(Some(Value::Bool(false))));
    }
}
