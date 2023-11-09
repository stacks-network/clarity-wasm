use std::cell::OnceCell;

use crate::wasm_generator::{
    clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError, WasmGenerator,
};
use clarity::vm::{
    types::{
        signatures::CallableSubtype, SequenceSubtype, StringSubtype, TupleTypeSignature,
        TypeSignature,
    },
    ClarityName, SymbolicExpression,
};
use walrus::{
    ir::{BinaryOp, IfElse, UnaryOp},
    InstrSeqBuilder, LocalId, ValType,
};

use super::Word;

#[derive(Debug)]
pub struct IsEq;

impl Word for IsEq {
    fn name(&self) -> ClarityName {
        "is-eq".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // Traverse the first operand pushing it onto the stack
        let first_op = args.get_expr(0)?;
        generator.traverse_expr(builder, first_op)?;
        let ty = generator
            .get_expr_type(first_op)
            .expect("is-eq value expression must be typed")
            .clone();

        // No need to go further if there is only one argument
        if args.is_empty() {
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
                .expect("is-eq value expression must be typed");
            assign_to_locals(builder, &ty, operand_ty, &nth_locals)?;

            // check equality
            wasm_equal(&ty, generator, builder, &val_locals, &nth_locals)?;

            // Do an "and" operation with the result from the previous function call.
            builder.binop(BinaryOp::I32And);
        }

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
    match dbg!(ty) {
        // we should never compare NoType
        TypeSignature::NoType => {
            builder.unreachable();
            Ok(())
        }
        // is-eq-int function can be reused to both int and uint types.
        TypeSignature::IntType | TypeSignature::UIntType => {
            wasm_equal_int128(generator, builder, first_op, nth_op)
        }
        // is-eq-bytes function can be used for types with (offset, length)
        TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
        | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_)))
        | TypeSignature::PrincipalType
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
    let func = OnceCell::new();
    builder.call(*func.get_or_init(|| generator.func_by_name("stdlib.is-eq-int")));

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
    let func = OnceCell::new();
    builder.call(*func.get_or_init(|| generator.func_by_name("stdlib.is-eq-bytes")));

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
    // [then]: we check if we have a `none` or if the `some` inner_type are equal
    // [else]: we push "false" on the stack
    let then_id = {
        let mut then = builder.dangling_instr_seq(ValType::I32);
        // is none ?
        then.local_get(*first_variant).unop(UnaryOp::I32Eqz);
        // is some inner equal ?
        wasm_equal(some_ty, generator, &mut then, first_inner, nth_inner)?; // is some arguments equal ?
        then.binop(BinaryOp::I32Or);
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
    let split_ok_err_idx = dbg!(clar2wasm_ty(ok_ty)).len();
    let Some((first_variant, first_ok, first_err)) =
        first_op.split_first().map(|(variant, rest)| {
            let (ok, err) = dbg!(rest.split_at(split_ok_err_idx));
            (variant, ok, err)
        })
    else {
        return Err(GeneratorError::InternalError(
            "Response operand should have at least one argument".into(),
        ));
    };
    let Some((nth_variant, nth_ok, nth_err)) = nth_op.split_first().map(|(variant, rest)| {
        let (ok, err) = dbg!(rest.split_at(split_ok_err_idx));
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
        inner_if
            .local_get(*first_variant)
            // 0 is err
            .unop(UnaryOp::I32Eqz)
            .instr(IfElse {
                consequent: err_id,
                alternative: ok_id,
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
        let (ty, range) = wasm_ranges.next().unwrap();
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
        let (ty, range) = wasm_ranges.next().unwrap();

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
        let (ty, range) = wasm_ranges.next().unwrap();
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
    let (ty, range) = wasm_ranges.next().unwrap();
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
