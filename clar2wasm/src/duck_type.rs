use clarity::vm::types::{SequenceSubtype, TypeSignature};
use walrus::{
    ir::{BinaryOp, Loop},
    InstrSeqBuilder, LocalId, ValType,
};

use crate::{
    wasm_generator::{
        add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, GeneratorError, WasmGenerator,
    },
    wasm_utils::get_type_in_memory_size,
};

impl WasmGenerator {
    /// Converts the representation of a Value from a type to another type. The Value keeps the
    /// same value in the end, only its representation in locals and memory differs.
    ///
    /// The original and target types should be "somewhat compatible" and validated by the typechecker
    /// for this function to succeed.
    #[allow(dead_code)]
    pub(crate) fn duck_type(
        &mut self,
        builder: &mut InstrSeqBuilder,
        og_ty: &TypeSignature,
        target_ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let former_stack_pointer = {
            let needed_workspace = dt_needed_workspace(target_ty);
            (needed_workspace > 0).then(|| {
                self.ensure_work_space(needed_workspace);
                let pointer = self.borrow_local(ValType::I32);
                builder.global_get(self.stack_pointer).local_set(*pointer);
                pointer
            })
        };

        let locals = self.create_locals_for_ty(target_ty);
        self.duck_type_stack(builder, og_ty, target_ty, &locals)?;
        for l in locals {
            builder.local_get(l);
        }

        if let Some(pointer) = former_stack_pointer {
            builder.local_get(*pointer).global_set(self.stack_pointer);
        }

        Ok(())
    }

    fn duck_type_stack(
        &mut self,
        builder: &mut InstrSeqBuilder,
        og_ty: &TypeSignature,
        target_ty: &TypeSignature,
        locals: &[LocalId],
    ) -> Result<(), GeneratorError> {
        match (og_ty, target_ty) {
            (TypeSignature::NoType, _) | (_, TypeSignature::NoType) => {
                // drop the useless original value from the stack
                drop_value(builder, og_ty);
                // set locals to zero values (needed for ducktying of list elements)
                add_placeholder_for_clarity_type(builder, target_ty);
                for &l in locals.iter().rev() {
                    builder.local_set(l);
                }
            }
            (TypeSignature::BoolType, TypeSignature::BoolType)
            | (TypeSignature::IntType, TypeSignature::IntType)
            | (TypeSignature::UIntType, TypeSignature::UIntType)
            | (TypeSignature::PrincipalType, TypeSignature::PrincipalType)
            | (
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_)),
                TypeSignature::SequenceType(SequenceSubtype::BufferType(_)),
            )
            | (
                TypeSignature::SequenceType(SequenceSubtype::StringType(_)),
                TypeSignature::SequenceType(SequenceSubtype::StringType(_)),
            )
            | (TypeSignature::CallableType(_), TypeSignature::CallableType(_))
            | (TypeSignature::TraitReferenceType(_), TypeSignature::TraitReferenceType(_)) => {
                for &l in locals.iter().rev() {
                    builder.local_set(l);
                }
            }
            (TypeSignature::OptionalType(og_subty), TypeSignature::OptionalType(target_subty)) => {
                let (variant_local, sub_locals) = locals.split_first().ok_or_else(|| {
                    GeneratorError::InternalError(
                        "Not enough locals for duck-typing an optional".to_owned(),
                    )
                })?;
                self.duck_type_stack(builder, og_subty, target_subty, sub_locals)?;
                builder.local_set(*variant_local);
            }
            (TypeSignature::ResponseType(og_subty), TypeSignature::ResponseType(target_subty)) => {
                let (og_ok_ty, og_err_ty) = og_subty.as_ref();
                let (target_ok_ty, target_err_ty) = target_subty.as_ref();

                let (variant_local, inner_locals) = locals.split_first().ok_or_else(|| {
                    GeneratorError::InternalError(
                        "Not enough locals for duck-typing a response".to_owned(),
                    )
                })?;
                let (ok_locals, err_locals) = inner_locals
                    .split_at_checked(clar2wasm_ty(target_ok_ty).len())
                    .ok_or_else(|| {
                        GeneratorError::InternalError(
                            "Not enough locals for duck-typing a response".to_owned(),
                        )
                    })?;

                self.duck_type_stack(builder, og_err_ty, target_err_ty, err_locals)?;
                self.duck_type_stack(builder, og_ok_ty, target_ok_ty, ok_locals)?;
                builder.local_set(*variant_local);
            }
            (TypeSignature::TupleType(og_tup_ty), TypeSignature::TupleType(target_tup_ty)) => {
                let og_ty_iter = og_tup_ty.get_type_map().values().rev();
                let target_ty_iter = target_tup_ty.get_type_map().values().rev();

                let mut remaining_locals = locals;
                for (og_subty, target_subty) in og_ty_iter.zip(target_ty_iter) {
                    let current_locals;
                    (remaining_locals, current_locals) = remaining_locals
                        .split_at_checked(remaining_locals.len() - clar2wasm_ty(target_subty).len())
                        .ok_or_else(|| {
                            GeneratorError::InternalError(
                                "Not enough locals for duck-typing a tuple".to_owned(),
                            )
                        })?;
                    self.duck_type_stack(builder, og_subty, target_subty, current_locals)?;
                }
            }
            (
                TypeSignature::SequenceType(SequenceSubtype::ListType(og_ltd)),
                TypeSignature::SequenceType(SequenceSubtype::ListType(target_ltd)),
            ) => {
                let og_elem_ty = og_ltd.get_list_item_type();
                let target_elem_ty = target_ltd.get_list_item_type();

                // Set list length and offset to locals, and reserve some working space to store the target elements
                let [offset, length] = locals else {
                    return Err(GeneratorError::InternalError(
                        "List duck typing should use only two locals".to_owned(),
                    ));
                };
                let offset_target = self.module.locals.add(ValType::I32);
                builder.local_set(*length);
                builder.local_set(*offset);

                // Create locals for the element target repr.
                let target_locs = self.create_locals_for_ty(target_elem_ty);

                // iterate through elements, convert them to target type and store them
                let loop_id = {
                    let mut loop_ = builder.dangling_instr_seq(None);
                    let loop_id = loop_.id();

                    let og_elem_size = self.read_from_memory(&mut loop_, *offset, 0, og_elem_ty)?;
                    self.duck_type_stack(&mut loop_, og_elem_ty, target_elem_ty, &target_locs)?;
                    for l in target_locs.iter().rev() {
                        loop_.local_get(*l);
                    }
                    let target_elem_size =
                        self.write_to_memory(&mut loop_, offset_target, 0, target_elem_ty)?;

                    loop_
                        .local_get(*offset)
                        .i32_const(og_elem_size)
                        .binop(BinaryOp::I32Add)
                        .local_set(*offset);
                    loop_
                        .local_get(offset_target)
                        .i32_const(target_elem_size as i32)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_target);
                    loop_
                        .local_get(*length)
                        .i32_const(1)
                        .binop(BinaryOp::I32Sub)
                        .local_tee(*length)
                        .br_if(loop_id);

                    loop_id
                };

                // we will "duck-type-clone" if the length of the list is not empty
                builder.local_get(*length).if_else(
                    None,
                    |then| {
                        // we set the offset_target to copy at the free space of stack-pointer and we move this on further
                        then.global_get(self.stack_pointer)
                            .local_tee(offset_target)
                            .i32_const(get_type_in_memory_size(target_ty, false))
                            .binop(BinaryOp::I32Add)
                            .global_set(self.stack_pointer);

                        // we put the resulting offset/length on the stack
                        then.local_get(offset_target).local_get(*length);

                        // the cloning loop
                        then.instr(Loop { seq: loop_id });

                        // we set the result back to the correct locals
                        then.local_set(*length).local_set(*offset);
                    },
                    |_else| {},
                );
            }
            (TypeSignature::ListUnionType(_), TypeSignature::ListUnionType(_)) => {
                return Err(GeneratorError::InternalError(
                    "Unconcretized ListUnionType".to_owned(),
                ))
            }
            _ => todo!(),
        }
        Ok(())
    }

    fn create_locals_for_ty(&mut self, ty: &TypeSignature) -> Vec<LocalId> {
        clar2wasm_ty(ty)
            .into_iter()
            .map(|vt| self.module.locals.add(vt))
            .collect()
    }
}

fn dt_needed_workspace(ty: &TypeSignature) -> u32 {
    match ty {
        TypeSignature::OptionalType(opt) => dt_needed_workspace(opt),
        TypeSignature::ResponseType(resp) => {
            dt_needed_workspace(&resp.0) + dt_needed_workspace(&resp.1)
        }
        TypeSignature::TupleType(tup) => tup.get_type_map().values().map(dt_needed_workspace).sum(),
        TypeSignature::SequenceType(SequenceSubtype::ListType(_)) => {
            get_type_in_memory_size(ty, false) as u32
        }
        _ => 0,
    }
}
