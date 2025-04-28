use clarity::vm::types::{SequenceSubtype, TypeSignature};
use walrus::ir::{BinaryOp, Loop};
use walrus::{InstrSeqBuilder, LocalId, ValType};

use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, GeneratorError, WasmGenerator,
};
use crate::wasm_utils::get_type_in_memory_size;

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

#[cfg(test)]
mod tests {
    use clarity::types::StacksEpochId;
    use clarity::vm::analysis::ContractAnalysis;
    use clarity::vm::costs::LimitedCostTracker;
    use clarity::vm::types::{
        QualifiedContractIdentifier, ResponseData, SequenceData, TupleData, TupleTypeSignature,
        TypeSignature,
    };
    use clarity::vm::{ClarityVersion, Value};
    use walrus::{FunctionBuilder, InstrSeqBuilder};
    use wasmtime::{Engine, Module, Store};

    use crate::linker::dummy_linker;
    use crate::wasm_generator::{
        add_placeholder_for_clarity_type, clar2wasm_ty, GeneratorError, WasmGenerator,
    };
    use crate::wasm_utils::{placeholder_for_type, wasm_to_clarity_value};

    impl WasmGenerator {
        fn empty() -> Self {
            let empty_analysis = ContractAnalysis::new(
                QualifiedContractIdentifier::transient(),
                vec![],
                LimitedCostTracker::Free,
                StacksEpochId::latest(),
                ClarityVersion::latest(),
            );
            WasmGenerator::new(empty_analysis)
                .expect("failed to build WasmGenerator for empty contract")
        }

        // TODO: this function shouldn't exist like this. As soon as #610 is done, we can
        // replace it by a simple `write_to_wasm` call followed by `WasmGenerator::read_from_memory`.
        fn pass_value(
            &mut self,
            builder: &mut InstrSeqBuilder,
            value: &Value,
            ty: &TypeSignature,
        ) -> Result<(), GeneratorError> {
            match value {
                Value::Bool(b) => {
                    builder.i32_const(*b as i32);
                    Ok(())
                }
                Value::Int(i) => {
                    builder.i64_const((i & 0xFFFFFFFFFFFFFFFF) as i64);
                    builder.i64_const(((i >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                    Ok(())
                }
                Value::UInt(u) => {
                    builder.i64_const((u & 0xFFFFFFFFFFFFFFFF) as i64);
                    builder.i64_const(((u >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                    Ok(())
                }
                Value::Sequence(SequenceData::String(s)) => {
                    let (offset, len) = self.add_clarity_string_literal(s)?;
                    builder.i32_const(offset as i32);
                    builder.i32_const(len as i32);
                    Ok(())
                }
                Value::Principal(_) | Value::Sequence(SequenceData::Buffer(_)) => {
                    let (offset, len) = self.add_literal(value)?;
                    builder.i32_const(offset as i32);
                    builder.i32_const(len as i32);
                    Ok(())
                }
                Value::Optional(opt) => {
                    let TypeSignature::OptionalType(inner_ty) = ty else {
                        return Err(GeneratorError::InternalError(
                            "Mismatched value/type".to_owned(),
                        ));
                    };
                    match &opt.data {
                        Some(inner) => {
                            builder.i32_const(1);
                            self.pass_value(builder, inner, inner_ty)?;
                        }
                        None => {
                            builder.i32_const(0);
                            add_placeholder_for_clarity_type(builder, inner_ty);
                        }
                    }
                    Ok(())
                }
                Value::Response(resp) => {
                    let TypeSignature::ResponseType(resp_ty) = ty else {
                        return Err(GeneratorError::InternalError(
                            "Mismatched value/type".to_owned(),
                        ));
                    };
                    builder.i32_const(resp.committed as i32);
                    if resp.committed {
                        self.pass_value(builder, &resp.data, &resp_ty.0)?;
                        add_placeholder_for_clarity_type(builder, &resp_ty.1);
                    } else {
                        add_placeholder_for_clarity_type(builder, &resp_ty.0);
                        self.pass_value(builder, &resp.data, &resp_ty.1)?;
                    }
                    Ok(())
                }
                Value::Tuple(tuple) => {
                    let TypeSignature::TupleType(tuple_ty) = ty else {
                        return Err(GeneratorError::InternalError(
                            "Mismatched value/type".to_owned(),
                        ));
                    };
                    for (elem, elem_ty) in tuple
                        .data_map
                        .values()
                        .zip(tuple_ty.get_type_map().values())
                    {
                        self.pass_value(builder, elem, elem_ty)?;
                    }
                    Ok(())
                }
                Value::Sequence(SequenceData::List(_list)) => {
                    todo!("Need to complete #610 first")
                }
                Value::CallableContract(_) => unimplemented!("We can already test principals"),
            }
        }

        fn create_duck_type_module(
            &mut self,
            value: &Value,
            original_type: &TypeSignature,
            target_type: &TypeSignature,
        ) {
            let return_ty = clar2wasm_ty(target_type);
            let mut top_level = FunctionBuilder::new(&mut self.module.types, &[], &return_ty);
            {
                let builder = &mut top_level.func_body();

                self.pass_value(builder, value, original_type)
                    .expect("failed to write instructions for original value");

                self.duck_type(builder, original_type, target_type)
                    .expect("failed to write duck type instructions");
            }

            let top_level = top_level.finish(vec![], &mut self.module.funcs);
            self.module.exports.add(".top-level", top_level);

            self.module.globals.get_mut(self.stack_pointer).kind =
                walrus::GlobalKind::Local(walrus::InitExpr::Value(walrus::ir::Value::I32(20000)));
        }

        fn execute_duck_type_module(&mut self, target_type: &TypeSignature) -> Value {
            std::fs::write("debug.wasm", self.module.emit_wasm()).unwrap();

            let engine = Engine::default();
            let mut store = Store::new(&engine, ());
            let linker = dummy_linker(&engine).expect("failed to create linker");
            let module =
                Module::new(&engine, self.module.emit_wasm()).expect("failed to create module");
            let instance = linker
                .instantiate(&mut store, &module)
                .expect("failed to instanciate module");

            let top_level = instance
                .get_func(&mut store, ".top-level")
                .expect("cannot find .top-level function");

            let mut result: Vec<_> = top_level
                .ty(&mut store)
                .results()
                .map(placeholder_for_type)
                .collect();

            top_level
                .call(&mut store, &[], &mut result)
                .expect("couldn't call .top-level");

            let memory = instance
                .get_memory(&mut store, "memory")
                .expect("couldn't find memory");
            wasm_to_clarity_value(
                target_type,
                0,
                &result,
                memory,
                &mut store,
                StacksEpochId::latest(),
            )
            .expect("error in execution")
            .0
            .expect("no value computed???")
        }
    }

    fn duck_type_test(value: Value, og_ty: TypeSignature, target_ty: TypeSignature) {
        let mut gen = WasmGenerator::empty();
        gen.create_duck_type_module(&value, &og_ty, &target_ty);
        let res = gen.execute_duck_type_module(&target_ty);

        assert_eq!(value, res);
    }

    #[test]
    fn duck_type_optional_int() {
        let value = Value::none();
        let og_ty = TypeSignature::OptionalType(Box::new(TypeSignature::NoType));
        let target_ty = TypeSignature::OptionalType(Box::new(TypeSignature::IntType));

        duck_type_test(value, og_ty, target_ty);
    }

    #[test]
    fn duck_type_optional_string() {
        let value = Value::none();
        let og_ty = TypeSignature::OptionalType(Box::new(TypeSignature::NoType));
        let target_ty = TypeSignature::OptionalType(Box::new(TypeSignature::SequenceType(
            clarity::vm::types::SequenceSubtype::StringType(
                clarity::vm::types::StringSubtype::ASCII(999u32.try_into().unwrap()),
            ),
        )));

        duck_type_test(value, og_ty, target_ty);
    }

    #[test]
    fn duck_type_response_int_int_from_ok() {
        let value = Value::okay(Value::Int(42)).unwrap();
        let og_ty =
            TypeSignature::ResponseType(Box::new((TypeSignature::IntType, TypeSignature::NoType)));
        let target_ty =
            TypeSignature::ResponseType(Box::new((TypeSignature::IntType, TypeSignature::IntType)));

        duck_type_test(value, og_ty, target_ty);
    }

    #[test]
    fn duck_type_response_int_int_from_err() {
        let value = Value::error(Value::Int(42)).unwrap();
        let og_ty =
            TypeSignature::ResponseType(Box::new((TypeSignature::NoType, TypeSignature::IntType)));
        let target_ty =
            TypeSignature::ResponseType(Box::new((TypeSignature::IntType, TypeSignature::IntType)));

        duck_type_test(value, og_ty, target_ty);
    }

    #[test]
    fn duck_type_response_string_int_from_ok() {
        let value = Value::okay(Value::string_ascii_from_bytes("hello".bytes().collect()).unwrap())
            .unwrap();
        let og_ty = TypeSignature::ResponseType(Box::new((
            TypeSignature::SequenceType(clarity::vm::types::SequenceSubtype::StringType(
                clarity::vm::types::StringSubtype::ASCII(42u32.try_into().unwrap()),
            )),
            TypeSignature::NoType,
        )));
        let target_ty = TypeSignature::ResponseType(Box::new((
            TypeSignature::SequenceType(clarity::vm::types::SequenceSubtype::StringType(
                clarity::vm::types::StringSubtype::ASCII(42u32.try_into().unwrap()),
            )),
            TypeSignature::IntType,
        )));

        duck_type_test(value, og_ty, target_ty);
    }

    #[test]
    fn duck_type_response_string_int_from_err() {
        let value = Value::error(Value::Int(42)).unwrap();
        let og_ty =
            TypeSignature::ResponseType(Box::new((TypeSignature::NoType, TypeSignature::IntType)));
        let target_ty = TypeSignature::ResponseType(Box::new((
            TypeSignature::SequenceType(clarity::vm::types::SequenceSubtype::StringType(
                clarity::vm::types::StringSubtype::ASCII(42u32.try_into().unwrap()),
            )),
            TypeSignature::IntType,
        )));

        duck_type_test(value, og_ty, target_ty);
    }

    #[test]
    fn duck_type_tuple() {
        let value = Value::from(
            TupleData::from_data(vec![
                ("a".into(), Value::Int(42)),
                ("b".into(), Value::none()),
                ("c".into(), Value::buff_from(vec![1, 2, 3, 4, 5]).unwrap()),
                (
                    "d".into(),
                    Value::Response(ResponseData {
                        committed: true,
                        data: Box::new(Value::none()),
                    }),
                ),
            ])
            .unwrap(),
        );
        let og_ty = TypeSignature::TupleType(
            TupleTypeSignature::try_from(vec![
                ("a".into(), TypeSignature::IntType),
                (
                    "b".into(),
                    TypeSignature::OptionalType(Box::new(TypeSignature::NoType)),
                ),
                (
                    "c".into(),
                    TypeSignature::SequenceType(clarity::vm::types::SequenceSubtype::BufferType(
                        500u32.try_into().unwrap(),
                    )),
                ),
                (
                    "d".into(),
                    TypeSignature::ResponseType(Box::new((
                        TypeSignature::OptionalType(Box::new(TypeSignature::NoType)),
                        TypeSignature::NoType,
                    ))),
                ),
            ])
            .unwrap(),
        );
        let target_ty = TypeSignature::TupleType(
            TupleTypeSignature::try_from(vec![
                ("a".into(), TypeSignature::IntType),
                (
                    "b".into(),
                    TypeSignature::OptionalType(Box::new(TypeSignature::UIntType)),
                ),
                (
                    "c".into(),
                    TypeSignature::SequenceType(clarity::vm::types::SequenceSubtype::BufferType(
                        500u32.try_into().unwrap(),
                    )),
                ),
                (
                    "d".into(),
                    TypeSignature::ResponseType(Box::new((
                        TypeSignature::OptionalType(Box::new(TypeSignature::IntType)),
                        TypeSignature::BoolType,
                    ))),
                ),
            ])
            .unwrap(),
        );

        duck_type_test(value, og_ty, target_ty);
    }
}
