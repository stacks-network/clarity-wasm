use clarity::vm::clarity_wasm::{get_type_size, PRINCIPAL_BYTES, STANDARD_PRINCIPAL_BYTES};
use clarity::vm::types::serialization::TypePrefix;
use clarity::vm::types::{
    ListTypeData, SequenceSubtype, StringSubtype, TupleTypeSignature, TypeSignature,
};
use walrus::ir::{
    BinaryOp, Block, Const, ExtendedLoad, IfElse, InstrSeqType, LoadKind, Loop, MemArg, StoreKind,
    UnaryOp,
};
use walrus::{InstrSeqBuilder, LocalId, MemoryId, ValType};

use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, GeneratorError, WasmGenerator,
};

impl WasmGenerator {
    /// Deserialize an integer (`int` or `uint`) from memory using consensus
    /// serialization. Leaves an `(optional int|uint)` on the top of the stack.
    /// See SIP-005 for details.
    ///
    /// Representation:
    ///   Int:
    ///     | 0x00 | value: 16-bytes (big-endian) |
    ///   UInt:
    ///     | 0x01 | value: 16-bytes (big-endian) |
    fn deserialize_integer(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        signed: bool,
    ) -> Result<(), GeneratorError> {
        // Create a block that returns `none` if 17 bytes from offset is
        // beyond the end of the buffer.
        let block_ty = InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32, ValType::I64, ValType::I64],
        );
        let mut none_block = builder.dangling_instr_seq(block_ty);

        // Return `none`
        none_block.i32_const(0).i64_const(0).i64_const(0);
        let none_block_id = none_block.id();

        // Create a block that continues to process the buffer if the length is
        // 17 bytes.
        let mut continue_block = builder.dangling_instr_seq(block_ty);
        let continue_block_id = continue_block.id();

        // Read the prefix byte
        continue_block.local_get(offset_local).load(
            memory,
            LoadKind::I32_8 {
                kind: ExtendedLoad::ZeroExtend,
            },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Verify the prefix byte
        continue_block
            .i32_const(if signed {
                TypePrefix::Int
            } else {
                TypePrefix::UInt
            } as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                block_ty,
                |then| {
                    // Push the `some` indicator onto the stack
                    then.i32_const(1);

                    // Load the integer into a vector register
                    then.local_get(offset_local).load(
                        memory,
                        LoadKind::V128 {},
                        MemArg {
                            align: 1,
                            offset: 1,
                        },
                    );
                    // Convert from big-endian to little
                    let tmp_v128 = self.module.locals.add(ValType::V128);
                    then.instr(Const {
                        value: walrus::ir::Value::V128(0x000102030405060708090a0b0c0d0e0f),
                    })
                    .i8x16_swizzle()
                    .local_tee(tmp_v128);

                    // Push the two i64s onto the stack
                    then.unop(UnaryOp::I64x2ExtractLane { idx: 0 });
                    then.local_get(tmp_v128)
                        .unop(UnaryOp::I64x2ExtractLane { idx: 1 });

                    // Increment the offset by 17
                    then.local_get(offset_local)
                        .i32_const(17)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_local);
                },
                |else_| {
                    // Return `none`
                    else_.i32_const(0).i64_const(0).i64_const(0);
                },
            );

        // Verify that reading 17 bytes from the offset is within the buffer
        builder
            .local_get(offset_local)
            .i32_const(17)
            .binop(BinaryOp::I32Add)
            .local_get(end_local)
            .binop(BinaryOp::I32GtU)
            .instr(IfElse {
                consequent: none_block_id,
                alternative: continue_block_id,
            });

        Ok(())
    }

    /// Deserialize a `principal` from memory using consensus serialization.
    /// Leaves an `(optional principal)` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///   Standard:
    ///    | 0x05 | version: 1 byte | public key(s)' hash160: 20-bytes |
    ///   Contract:
    ///    | 0x06 | version: 1 byte | public key(s)' hash160: 20-bytes
    ///      | contract name length: 1 byte | contract name: variable length |
    fn deserialize_principal(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset_result: LocalId,
        end_local: LocalId,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        let block_ty = InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32, ValType::I32, ValType::I32],
        );
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 17 bytes from the offset is within the buffer
        block
            .local_get(offset_local)
            .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
            .binop(BinaryOp::I32Add)
            .local_get(end_local)
            .binop(BinaryOp::I32GtU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        let type_prefix = self.module.locals.add(ValType::I32);
        block
            .local_get(offset_local)
            .load(
                memory,
                LoadKind::I32_8 {
                    kind: ExtendedLoad::ZeroExtend,
                },
                MemArg {
                    align: 1,
                    offset: 0,
                },
            )
            .local_tee(type_prefix);

        // Check for the standard principal prefix (0x05)
        block
            .i32_const(TypePrefix::PrincipalStandard as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                None,
                |then| {
                    // Push the `some` indicator onto the stack
                    then.i32_const(1);

                    // Allocate space for the principal on the call stack
                    let principal_offset = self.module.locals.add(ValType::I32);
                    then.local_get(offset_result).local_tee(principal_offset);
                    then.i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_result);

                    // Copy the principal to the destination
                    then.local_get(principal_offset)
                        .local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .i32_const(PRINCIPAL_BYTES as i32)
                        .memory_copy(memory, memory);

                    // Write the contract name length (0)
                    then.local_get(principal_offset).i32_const(0).store(
                        memory,
                        StoreKind::I32_8 { atomic: false },
                        MemArg {
                            align: 1,
                            offset: PRINCIPAL_BYTES as u32,
                        },
                    );

                    // Increment the offset by the length of the serialized
                    // principal.
                    then.local_get(offset_local)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_local);

                    // Push the offset and length onto the stack
                    then.local_get(principal_offset)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32);

                    // Break out of the block
                    then.br(block_id);
                },
                |_| {},
            );

        // Check for the contract principal prefix (0x06)
        block
            .local_get(type_prefix)
            .i32_const(TypePrefix::PrincipalContract as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                block_ty,
                |then| {
                    // Read the contract name length
                    let contract_length = self.module.locals.add(ValType::I32);
                    then.local_get(offset_local)
                        .load(
                            memory,
                            LoadKind::I32_8 {
                                kind: ExtendedLoad::ZeroExtend,
                            },
                            MemArg {
                                align: 1,
                                offset: STANDARD_PRINCIPAL_BYTES as u32,
                            },
                        )
                        .local_tee(contract_length);

                    // Verify that the contract name length is within the
                    // buffer.
                    let computed_end = self.module.locals.add(ValType::I32);
                    then.local_get(offset_local)
                        .binop(BinaryOp::I32Add)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32 + 1)
                        .binop(BinaryOp::I32Add)
                        .local_tee(computed_end)
                        .local_get(end_local)
                        .binop(BinaryOp::I32GtU)
                        .if_else(
                            None,
                            |inner| {
                                // Return none
                                inner.i32_const(0).i32_const(0).i32_const(0);
                                inner.br(block_id);
                            },
                            |_| {},
                        );

                    // Push the `some` indicator onto the stack
                    then.i32_const(1);

                    // The serialized principal is represented in the same
                    // way that Clarity-Wasm expects, after the type prefix
                    // so just return a pointer to the serialized principal.
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add);

                    // The total length is the contract name length plus
                    // the standard principal length.
                    then.local_get(contract_length)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Add);

                    // Increment the offset by the length of the serialized
                    // principal.
                    then.local_get(computed_end).local_set(offset_local);
                },
                |else_| {
                    // Invalid prefix, return `none`.
                    else_.i32_const(0).i32_const(0).i32_const(0);
                },
            );

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize a `bool` from memory using consensus serialization.
    /// Leaves an `(optional bool)` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///   True:
    ///    | 0x03 |
    ///   False:
    ///    | 0x04 |
    fn deserialize_bool(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        let block_ty =
            InstrSeqType::new(&mut self.module.types, &[], &[ValType::I32, ValType::I32]);
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 1 bytes from the offset is within the buffer
        block
            .local_get(offset_local)
            .local_get(end_local)
            .binop(BinaryOp::I32GeU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        let type_prefix = self.module.locals.add(ValType::I32);
        block
            .local_get(offset_local)
            .load(
                memory,
                LoadKind::I32_8 {
                    kind: ExtendedLoad::ZeroExtend,
                },
                MemArg {
                    align: 1,
                    offset: 0,
                },
            )
            .local_tee(type_prefix);

        // Check for the `true` prefix (0x03)
        block
            .i32_const(TypePrefix::BoolTrue as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                None,
                |then| {
                    // Push `(some true)` onto the stack
                    then.i32_const(1).i32_const(1);

                    // Increment the offset by 1.
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_local);

                    // Break out of the block
                    then.br(block_id);
                },
                |_| {},
            );

        // Check for the `false` prefix (0x04)
        block
            .local_get(type_prefix)
            .i32_const(TypePrefix::BoolFalse as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                block_ty,
                |then| {
                    // Push `(some false)` onto the stack
                    then.i32_const(1).i32_const(0);

                    // Increment the offset by 1.
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_local);
                },
                |else_| {
                    // Invalid prefix, return `none`.
                    else_.i32_const(0).i32_const(0);
                },
            );

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize an `optional` from memory using consensus serialization.
    /// Leaves an `(optional (optional T))` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///   None:
    ///    | 0x09 |
    ///   Some:
    ///    | 0x0a | serialized value |
    fn deserialize_optional(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        offset_result: LocalId,
        value_ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        // These two I32's are the some indicators for the outer and inner
        // optionals.
        let mut wasm_val_ty = vec![ValType::I32, ValType::I32];
        wasm_val_ty.append(&mut clar2wasm_ty(value_ty));
        let block_ty = InstrSeqType::new(&mut self.module.types, &[], &wasm_val_ty);
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 1 bytes from the offset is within the buffer
        block
            .local_get(offset_local)
            .local_get(end_local)
            .binop(BinaryOp::I32GeU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0);
                    add_placeholder_for_clarity_type(then, value_ty);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        let type_prefix = self.module.locals.add(ValType::I32);
        block
            .local_get(offset_local)
            .load(
                memory,
                LoadKind::I32_8 {
                    kind: ExtendedLoad::ZeroExtend,
                },
                MemArg {
                    align: 1,
                    offset: 0,
                },
            )
            .local_tee(type_prefix);

        // Check for the `none` prefix (0x09)
        block
            .i32_const(TypePrefix::OptionalNone as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                None,
                |then| {
                    // Push `(some none)` onto the stack (with a placeholder for
                    // the inner type).
                    then.i32_const(1).i32_const(0);
                    add_placeholder_for_clarity_type(then, value_ty);

                    // Increment the offset by 1.
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .local_set(offset_local);

                    // Break out of the block
                    then.br(block_id);
                },
                |_| {},
            );

        // Check for the `some` prefix (0x0a)

        // Build the block for the case where the prefix is `some`
        let mut some_block = block.dangling_instr_seq(block_ty);
        let some_block_id = some_block.id();

        // Increment offset by 1
        some_block
            .local_get(offset_local)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_set(offset_local);

        // Deserialize the inner value
        self.deserialize_from_memory(
            &mut some_block,
            offset_local,
            end_local,
            offset_result,
            value_ty,
        )?;

        // Check if the deserialization failed:
        // - Store the value in locals
        // - Check the indicator now on top of the stack
        let inner_locals = self.save_to_locals(&mut some_block, value_ty, true);
        some_block.unop(UnaryOp::I32Eqz).if_else(
            None,
            |then| {
                // Return none
                then.i32_const(0).i32_const(0);
                add_placeholder_for_clarity_type(then, value_ty);
                then.br(block_id);
            },
            |_| {},
        );

        // Push the `some` indicator onto the stack, for the result of this
        // operation, then push the `some` indicator for the actual value
        // we deserialized.
        some_block.i32_const(1).i32_const(1);

        // Push the inner value back onto the stack
        for local in inner_locals {
            some_block.local_get(local);
        }

        // Build the block for the case of an invalid type prefix
        let mut invalid_block = block.dangling_instr_seq(block_ty);
        let invalid_block_id = invalid_block.id();

        // Invalid prefix, return `none`.
        invalid_block.i32_const(0).i32_const(0);
        add_placeholder_for_clarity_type(&mut invalid_block, value_ty);

        // Check for the `some` prefix (0x0a)
        block
            .local_get(type_prefix)
            .i32_const(TypePrefix::OptionalSome as i32)
            .binop(BinaryOp::I32Eq)
            .instr(IfElse {
                consequent: some_block_id,
                alternative: invalid_block_id,
            });

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize a `response` from memory using consensus serialization.
    /// Leaves an `(optional (response O E))` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///   Ok:
    ///    | 0x07 | serialized ok value |
    ///   Err:
    ///    | 0x08 | serialized err value |
    #[allow(clippy::too_many_arguments)]
    fn deserialize_response(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        offset_result: LocalId,
        ok_ty: &TypeSignature,
        err_ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        // These two I32's are the some indicator for the outer optional and
        // the ok/err indicator for the inner response.
        let mut wasm_val_ty = vec![ValType::I32, ValType::I32];
        wasm_val_ty.append(&mut clar2wasm_ty(ok_ty));
        wasm_val_ty.append(&mut clar2wasm_ty(err_ty));
        let block_ty = InstrSeqType::new(&mut self.module.types, &[], &wasm_val_ty);
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 1 bytes from the offset is within the buffer
        block
            .local_get(offset_local)
            .local_get(end_local)
            .binop(BinaryOp::I32GeU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0);
                    add_placeholder_for_clarity_type(then, ok_ty);
                    add_placeholder_for_clarity_type(then, err_ty);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        let type_prefix = self.module.locals.add(ValType::I32);
        block
            .local_get(offset_local)
            .load(
                memory,
                LoadKind::I32_8 {
                    kind: ExtendedLoad::ZeroExtend,
                },
                MemArg {
                    align: 1,
                    offset: 0,
                },
            )
            .local_tee(type_prefix);

        // Build the block for the case where the prefix is `ok`
        let mut ok_block = block.dangling_instr_seq(block_ty);
        let ok_block_id = ok_block.id();

        // Increment offset by 1
        ok_block
            .local_get(offset_local)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_set(offset_local);

        // Deserialize the inner value
        self.deserialize_from_memory(&mut ok_block, offset_local, end_local, offset_result, ok_ty)?;

        // Check if the deserialization failed:
        // - Store the value in locals
        // - Check the inidicator now on top of the stack
        let inner_locals = self.save_to_locals(&mut ok_block, ok_ty, true);
        ok_block.unop(UnaryOp::I32Eqz).if_else(
            None,
            |then| {
                // Return none
                then.i32_const(0).i32_const(0);
                add_placeholder_for_clarity_type(then, ok_ty);
                add_placeholder_for_clarity_type(then, err_ty);
                then.br(block_id);
            },
            |_| {},
        );

        // Push the `some` indicator onto the stack, for the result of this
        // operation, then push the `ok` indicator for the actual value
        // we deserialized.
        ok_block.i32_const(1).i32_const(1);

        // Push the inner value back onto the stack
        for local in inner_locals {
            ok_block.local_get(local);
        }

        // Push a placeholder for the err value
        add_placeholder_for_clarity_type(&mut ok_block, err_ty);

        // Build the block for the case where the prefix is `err`
        let mut err_block = block.dangling_instr_seq(block_ty);
        let err_block_id = err_block.id();

        // Check for the `err` prefix (0x08)
        err_block
            .local_get(type_prefix)
            .i32_const(TypePrefix::ResponseErr as i32)
            .binop(BinaryOp::I32Ne)
            .if_else(
                None,
                |then| {
                    // Return none, since this is not an 'ok' or 'err' prefix
                    then.i32_const(0).i32_const(0);
                    add_placeholder_for_clarity_type(then, ok_ty);
                    add_placeholder_for_clarity_type(then, err_ty);
                    then.br(block_id);
                },
                |_| {},
            );

        // Increment offset by 1
        err_block
            .local_get(offset_local)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_set(offset_local);

        // Deserialize the inner value
        self.deserialize_from_memory(
            &mut err_block,
            offset_local,
            end_local,
            offset_result,
            err_ty,
        )?;

        // Check if the deserialization failed:
        // - Store the value in locals
        // - Check the inidicator now on top of the stack
        let inner_locals = self.save_to_locals(&mut err_block, err_ty, true);
        err_block.unop(UnaryOp::I32Eqz).if_else(
            None,
            |then| {
                // Return none
                then.i32_const(0).i32_const(0);
                add_placeholder_for_clarity_type(then, ok_ty);
                add_placeholder_for_clarity_type(then, err_ty);
                then.br(block_id);
            },
            |_| {},
        );

        // Push the `some` indicator onto the stack, for the result of this
        // operation, then push the `err` indicator for the actual value
        // we deserialized.
        err_block.i32_const(1).i32_const(0);

        // Push a placeholder for the ok value
        add_placeholder_for_clarity_type(&mut err_block, ok_ty);

        // Push the inner value back onto the stack
        for local in inner_locals {
            err_block.local_get(local);
        }

        // Check for the `ok` prefix (0x0a)
        block
            .i32_const(TypePrefix::ResponseOk as i32)
            .binop(BinaryOp::I32Eq)
            .instr(IfElse {
                consequent: ok_block_id,
                alternative: err_block_id,
            });

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize a `list` from memory using consensus serialization.
    /// Leaves an `(optional (list n T))` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///    | 0x0b | number of elements: 4-bytes (big-endian)
    ///         | serialized representation of element 0
    ///         | serialized representation of element 1
    ///         | ...
    fn deserialize_list(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        offset_result: LocalId,
        list_ty: &ListTypeData,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        // These I32s are the some indicator for the outer optional and
        // the offset and length of the list.
        let wasm_val_ty = vec![ValType::I32, ValType::I32, ValType::I32];
        let block_ty = InstrSeqType::new(&mut self.module.types, &[], &wasm_val_ty);
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 5 bytes (prefix + length) from the offset is
        // within the buffer.
        block
            .local_get(offset_local)
            .i32_const(5)
            .binop(BinaryOp::I32Add)
            .local_get(end_local)
            .binop(BinaryOp::I32GtU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        block.local_get(offset_local).load(
            memory,
            LoadKind::I32_8 {
                kind: ExtendedLoad::ZeroExtend,
            },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Verify the prefix byte
        block
            .i32_const(TypePrefix::List as i32)
            .binop(BinaryOp::I32Ne)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the length of the list
        let length = self.module.locals.add(ValType::I32);
        block
            .local_get(offset_local)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .call(self.func_by_name("stdlib.load-i32-be"))
            .local_tee(length);

        // Verify that the length is within the specified type
        block
            .i32_const(list_ty.get_max_len() as i32)
            .binop(BinaryOp::I32GtU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Allocate space for the list on the call stack
        let element_ty = list_ty.get_list_item_type();
        let result = self.module.locals.add(ValType::I32);
        let element_offset = self.module.locals.add(ValType::I32);
        let element_size = get_type_size(element_ty);
        block
            .local_get(offset_result)
            .local_tee(result)
            .local_tee(element_offset);
        block
            .local_get(length)
            .i32_const(element_size)
            .binop(BinaryOp::I32Mul)
            .binop(BinaryOp::I32Add)
            .local_set(offset_result);

        // Update the offset to point to the first element
        block
            .local_get(offset_local)
            .i32_const(5)
            .binop(BinaryOp::I32Add)
            .local_set(offset_local);

        // Initialize an index variable to 0
        let index = self.module.locals.add(ValType::I32);
        block.i32_const(0).local_set(index);

        // Loop and deserialize each element
        let mut loop_block = block.dangling_instr_seq(block_ty);
        let loop_block_id = loop_block.id();

        // Check if we've reached the end of the list
        loop_block
            .local_get(index)
            .local_get(length)
            .binop(BinaryOp::I32GeU)
            .if_else(
                None,
                |then| {
                    // Push the `some` indicator onto the stack
                    then.i32_const(1);

                    // Push the offset and length onto the stack
                    then.local_get(result)
                        .local_get(length)
                        .i32_const(element_size)
                        .binop(BinaryOp::I32Mul);

                    // Break out of the loop
                    then.br(block_id);
                },
                |_| {},
            );

        // Deserialize the element. Note, this will update the offset to point
        // to the next element.
        self.deserialize_from_memory(
            &mut loop_block,
            offset_local,
            end_local,
            offset_result,
            element_ty,
        )?;

        // Check if the deserialization failed:
        // - Store the value in locals
        // - Check the inidicator now on top of the stack
        let inner_locals = self.save_to_locals(&mut loop_block, element_ty, true);

        loop_block.unop(UnaryOp::I32Eqz).if_else(
            None,
            |then| {
                // Return none
                then.i32_const(0).i32_const(0).i32_const(0);
                then.br(block_id);
            },
            |_| {},
        );

        // Deserializing the element was successful, so add it to the list
        // by storing it in the result buffer:
        // - Load the element value back to the stack
        // - Write it into the result buffer
        for local in inner_locals {
            loop_block.local_get(local);
        }
        let bytes_written = self.write_to_memory(&mut loop_block, element_offset, 0, element_ty)?;

        // Increment the result offset by the number of bytes written
        loop_block
            .local_get(element_offset)
            .i32_const(bytes_written as i32)
            .binop(BinaryOp::I32Add)
            .local_set(element_offset);

        // Increment the index by 1
        loop_block
            .local_get(index)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_set(index);

        // Loop back to the start of the loop
        loop_block.br(loop_block_id);

        // Add the loop block to the builder
        block.instr(Loop { seq: loop_block_id });

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize a `tuple` from memory using consensus serialization.
    /// Leaves an `(optional (tuple ...))` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///  | 0x0c | number of keys: 4-bytes (big-endian)
    ///    | key 0 length: 1-byte | key 0: variable length | serialized value 0
    ///    ...
    ///    | key N length: 1-byte | key N: variable length | serialized value N
    fn deserialize_tuple(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        offset_result: LocalId,
        tuple_ty: &TupleTypeSignature,
    ) -> Result<(), GeneratorError> {
        // We need to be able to parse the keys coming in a random order, only one occurence of each key.
        // We should ignore a valid key and value that is not specified in the result type.
        // Here is what is generated in pseudo-code:
        //
        //     let bitset = Bitset::new();
        //     for key in serialized_bytes {
        //         let n = find_index(key)
        //         switch n {
        //             case 1:
        //                 if bitset.contains(key) { return None; }
        //                     handle_parsing_of_value_1;
        //                     bitset.insert(key);
        //                     break;
        //             case 2:
        //                 if bitset.contains(key) { return None; }
        //                     handle_parsing_of_value_2;
        //                     bitset.insert(key);
        //                     break;
        //             ...
        //             default:
        //                 check_valid_skippable_key();
        //                 check_valid_skippable_value();
        //                 break;
        //         }
        //     }
        //     if bitset.full() { return Some(result) } else { return None };
        //
        // We will need to add all the keys to the data to be able to check if
        // they are part of the tuple and find their index. They will be stored as
        // [number of keys as u32 | key 1 offset as u32 | key 2 offset as u32 | key 1 len as u8 | key 2 len as u8 | ... | key 1 | key 2 | ...]
        let tm = tuple_ty.get_type_map();
        let (keys_offset, keys_len) = {
            let mut keys = (tm.len() as u32).to_le_bytes().to_vec();
            // add relative offsets
            keys.extend(
                tm.keys()
                    .scan(
                        self.literal_memory_end + 4 + tm.len() as u32 * 5,
                        |state, name| {
                            let res = state.to_le_bytes();
                            *state += name.len() as u32;
                            Some(res)
                        },
                    )
                    .flatten(),
            );
            // add lens
            keys.extend(tm.keys().map(|name| name.len()));
            // add keys names
            keys.extend(tm.keys().flat_map(|name| name.as_bytes()));
            self.add_bytes_literal(&keys)?
        };

        let ty = TypeSignature::TupleType(tuple_ty.clone());

        // bitset which will indicate if a field was defined or not
        let result_len = tm.len();
        let bitset: Vec<LocalId> = (0..result_len.div_ceil(32))
            .map(|_| self.module.locals.add(ValType::I32))
            .collect();

        // locals that will hold the tuple values, in the same order as the tuple type map
        let values_locals: Vec<Vec<LocalId>> = tm
            .values()
            .map(|ty_| {
                clar2wasm_ty(ty_)
                    .into_iter()
                    .map(|local_ty| self.module.locals.add(local_ty))
                    .collect()
            })
            .collect();

        // This locale will contain the remaining number of fields to deserialize
        let remaining_fields = self.module.locals.add(ValType::I32);

        // Create a main block for the body of this operation, so that we can
        // early exit as needed.
        let mut wasm_val_ty = vec![ValType::I32];
        wasm_val_ty.extend(clar2wasm_ty(&ty));
        let return_ty = InstrSeqType::new(&mut self.module.types, &[], &wasm_val_ty);

        // Main block creation
        let main_block_id = {
            let mut main_block = builder.dangling_instr_seq(return_ty);

            // we initialize the empty bitset
            for &b in bitset.iter() {
                main_block.i32_const(0).local_set(b);
            }

            // Create the done_block which will contain the loop and all the blocks for the
            // switch-case-like construction.
            let done_block_id = {
                let mut done_block = main_block.dangling_instr_seq(None);
                let done_block_id = done_block.id();

                // Verify that reading 5 bytes (prefix + number of keys) from the
                // offset is within the buffer.
                done_block
                    .local_get(offset_local)
                    .i32_const(5)
                    .binop(BinaryOp::I32Add)
                    .local_get(end_local)
                    .binop(BinaryOp::I32GtU)
                    .br_if(done_block_id);

                // Read the prefix byte
                done_block.local_get(offset_local).load(
                    memory,
                    LoadKind::I32_8 {
                        kind: ExtendedLoad::ZeroExtend,
                    },
                    MemArg {
                        align: 1,
                        offset: 0,
                    },
                );

                // Verify the prefix byte
                done_block
                    .i32_const(TypePrefix::Tuple as i32)
                    .binop(BinaryOp::I32Ne)
                    .br_if(done_block_id);

                // Read the number of keys and check that it's >= to the
                // result tuple number of fields
                done_block
                    .local_get(offset_local)
                    .i32_const(1)
                    .binop(BinaryOp::I32Add)
                    .call(self.func_by_name("stdlib.load-i32-be"))
                    .local_tee(remaining_fields)
                    .i32_const(tuple_ty.get_type_map().len() as i32)
                    .binop(BinaryOp::I32LtU)
                    .br_if(done_block_id);

                // Update the offset to point to the first key
                done_block
                    .local_get(offset_local)
                    .i32_const(5)
                    .binop(BinaryOp::I32Add)
                    .local_set(offset_local);

                // This is the loop body, which will contain the switch/case
                let loop_id = {
                    let mut loop_ = done_block.dangling_instr_seq(None);
                    let loop_id = loop_.id();

                    // variable for the field name size
                    let name_size = self.module.locals.add(ValType::I32);

                    // Here are all the blocks needed for the switch-case
                    let switch_case_blocks: Vec<_> = (0..=tuple_ty.get_type_map().len())
                        .map(|_| loop_.dangling_instr_seq(None).id())
                        .collect();

                    // `switch_case_blocks` should be at least of length 2 since empty Tuple cannot exist.
                    if switch_case_blocks.len() < 2 {
                        return Err(GeneratorError::InternalError(
                            "Tuple should have a least one field".to_owned(),
                        ));
                    }

                    // Here is the switch
                    {
                        let mut switch_block = loop_.instr_seq(switch_case_blocks[0]);

                        // Check that we have one byte for the field name and fail if not.
                        switch_block
                            .local_get(offset_local)
                            .local_get(end_local)
                            .binop(BinaryOp::I32GeU)
                            .br_if(done_block_id);

                        // Load the number of bytes for the field name
                        switch_block
                            .local_get(offset_local)
                            .load(
                                memory,
                                LoadKind::I32_8 {
                                    kind: ExtendedLoad::ZeroExtend,
                                },
                                MemArg {
                                    align: 1,
                                    offset: 0,
                                },
                            )
                            .local_tee(name_size);

                        // Check that we have enough bytes left for the parsing the name and fail if not.
                        switch_block
                            .local_get(offset_local)
                            .i32_const(1)
                            .binop(BinaryOp::I32Add)
                            .local_tee(offset_local)
                            .binop(BinaryOp::I32Add)
                            .local_get(end_local)
                            .binop(BinaryOp::I32GtU)
                            .br_if(done_block_id);

                        // Compute the index of the field name to know which branch to take in
                        // the switch case.
                        switch_block
                            .i32_const(keys_offset as i32)
                            .i32_const(keys_len as i32)
                            .local_get(offset_local)
                            .local_get(name_size)
                            .call(self.func_by_name("stdlib.bsearch-clarity-name"));

                        // update the offset local to point after the field name
                        switch_block
                            .local_get(offset_local)
                            .local_get(name_size)
                            .binop(BinaryOp::I32Add)
                            .local_set(offset_local);

                        // branch to the correct case
                        #[allow(clippy::expect_used)]
                        let (default, blocks) = switch_case_blocks
                            .split_last()
                            .expect("blocks should have at least the default block");
                        switch_block.br_table(blocks.into(), *default);
                    }

                    // switch case for valid fields
                    for (((&case, field_ty), field_locals), case_idx) in switch_case_blocks[1..]
                        .iter()
                        .zip(tm.values())
                        .zip(values_locals.iter())
                        .zip(0usize..)
                    {
                        let mut case_block = loop_.instr_seq(case);

                        // link the previous block to this one
                        case_block.instr(Block {
                            seq: switch_case_blocks[case_idx],
                        });

                        // check in the bitset if we haven't already dealt with this type
                        let bitset_idx = case_idx / 32;
                        let bitset_pos = 1u32 << (case_idx % 32);
                        case_block
                            .local_get(bitset[bitset_idx])
                            .i32_const(bitset_pos as i32)
                            .binop(BinaryOp::I32And)
                            .br_if(done_block_id);

                        // try to deserialize the value and add the result to locals
                        self.deserialize_from_memory(
                            &mut case_block,
                            offset_local,
                            end_local,
                            offset_result,
                            field_ty,
                        )?;
                        for &l in field_locals.iter().rev() {
                            case_block.local_set(l);
                        }

                        // last value after deserialization is for success/failure
                        case_block.unop(UnaryOp::I32Eqz).br_if(done_block_id);

                        // we set the bit in the bitset
                        case_block
                            .local_get(bitset[bitset_idx])
                            .i32_const(bitset_pos as i32)
                            .binop(BinaryOp::I32Or)
                            .local_set(bitset[bitset_idx]);

                        // we loop if we still have fields to deserialize
                        case_block
                            .local_get(remaining_fields)
                            .i32_const(1)
                            .binop(BinaryOp::I32Sub)
                            .local_tee(remaining_fields)
                            .br_if(loop_id);

                        // otherwise it means we are done with the deserialization
                        case_block.br(done_block_id);
                    }

                    // default code, which is in the loop after all the cases. It's for an unknown
                    // field name.
                    #[allow(clippy::expect_used)]
                    loop_.instr(Block {
                        seq: *switch_case_blocks
                            .last()
                            .expect("blocks should always have the default block"),
                    });

                    // check for the validity of the field name
                    loop_
                        .local_get(offset_local)
                        .local_get(name_size)
                        .binop(BinaryOp::I32Sub)
                        .local_get(name_size)
                        .call(self.func_by_name("stdlib.check-clarity-name"))
                        .unop(UnaryOp::I32Eqz)
                        .br_if(done_block_id);

                    // we check if the field value is correct and skip it
                    loop_
                        .local_get(offset_local)
                        .local_get(end_local)
                        .call(self.func_by_name("stdlib.skip-unknown-value"))
                        .local_tee(offset_local)
                        .unop(UnaryOp::I32Eqz)
                        .br_if(done_block_id);

                    // we loop if we still have fields to deserialize
                    loop_
                        .local_get(remaining_fields)
                        .i32_const(1)
                        .binop(BinaryOp::I32Sub)
                        .local_tee(remaining_fields)
                        .br_if(loop_id);

                    loop_id
                };

                done_block.instr(Loop { seq: loop_id });
                done_block_id
            };

            main_block.instr(Block { seq: done_block_id });

            // check if there are no more keys to parse
            main_block.local_get(remaining_fields).unop(UnaryOp::I32Eqz);

            // check if the bitset is full
            #[allow(clippy::expect_used)]
            let (last, inits) = bitset
                .split_last()
                .expect("bitset cannot be empty since tuple cannot be 0-tuple");
            for &b in inits {
                main_block
                    .local_get(b)
                    .i32_const(u32::MAX as i32)
                    .binop(BinaryOp::I32Eq)
                    .binop(BinaryOp::I32And);
            }
            let bits_in_last = if result_len % 32 == 0 {
                u32::MAX as i32
            } else {
                (1u32 << (result_len % 32)).wrapping_sub(1) as i32
            };
            main_block
                .local_get(*last)
                .i32_const(bits_in_last)
                .binop(BinaryOp::I32Eq)
                .binop(BinaryOp::I32And);

            main_block.if_else(
                return_ty,
                |then| {
                    then.i32_const(1);
                    for l in values_locals.into_iter().flatten() {
                        then.local_get(l);
                    }
                },
                |else_| {
                    else_.i32_const(0);
                    add_placeholder_for_clarity_type(else_, &ty);
                },
            );

            main_block.id()
        };

        builder.instr(Block { seq: main_block_id });
        Ok(())
    }

    /// Deserialize a `buffer` from memory using consensus serialization.
    /// Leaves an `(optional buffer)` on the top of the data stack. See
    /// SIP-005 for details.
    ///
    /// Representation:
    ///  | 0x02 | length: 4-bytes (big-endian) | data: variable length |
    fn deserialize_buffer(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        type_length: u32,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        let block_ty = InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32, ValType::I32, ValType::I32],
        );
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 5 bytes from the offset is within the buffer
        block
            .local_get(offset_local)
            .i32_const(5)
            .binop(BinaryOp::I32Add)
            .local_get(end_local)
            .binop(BinaryOp::I32GtU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        block.local_get(offset_local).load(
            memory,
            LoadKind::I32_8 {
                kind: ExtendedLoad::ZeroExtend,
            },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Check for the buffer prefix (0x02)
        block
            .i32_const(TypePrefix::Buffer as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                block_ty,
                |then| {
                    // Read the buffer length
                    let buffer_length = self.module.locals.add(ValType::I32);
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .call(self.func_by_name("stdlib.load-i32-be"))
                        .local_tee(buffer_length);

                    // Verify that the buffer length is within the
                    // buffer type size.
                    then.i32_const(type_length as i32)
                        .binop(BinaryOp::I32GtU)
                        .if_else(
                            None,
                            |inner| {
                                // Return none
                                inner.i32_const(0).i32_const(0).i32_const(0);
                                inner.br(block_id);
                            },
                            |_| {},
                        );

                    // Verify that the buffer length is within the
                    // buffer.
                    let computed_end = self.module.locals.add(ValType::I32);
                    then.local_get(buffer_length)
                        .local_get(offset_local)
                        .binop(BinaryOp::I32Add)
                        .i32_const(5)
                        .binop(BinaryOp::I32Add)
                        .local_tee(computed_end)
                        .local_get(end_local)
                        .binop(BinaryOp::I32GtU)
                        .if_else(
                            None,
                            |inner| {
                                // Return none
                                inner.i32_const(0).i32_const(0).i32_const(0);
                                inner.br(block_id);
                            },
                            |_| {},
                        );

                    // Push the `some` indicator onto the stack
                    then.i32_const(1);

                    // The serialized buffer is represented in the same
                    // way that Clarity-Wasm expects, after the type prefix
                    // and size, so just return a pointer to the corresponding
                    // part of the serialized buffer.
                    then.local_get(offset_local)
                        .i32_const(5)
                        .binop(BinaryOp::I32Add);

                    // Push the buffer length onto the stack
                    then.local_get(buffer_length);

                    // Increment the offset by the length of the serialized
                    // buffer.
                    then.local_get(computed_end).local_set(offset_local);
                },
                |else_| {
                    // Invalid prefix, return `none`.
                    else_.i32_const(0).i32_const(0).i32_const(0);
                },
            );

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize a `string-ascii` from memory using consensus serialization.
    /// Leaves an `(optional (string-ascii n))` on the top of the data stack.
    /// See SIP-005 for details.
    ///
    /// Representation:
    ///  | 0x0d | length: 4-bytes (big-endian) | ascii-encoded string: variable length |
    fn deserialize_string_ascii(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        type_length: u32,
    ) -> Result<(), GeneratorError> {
        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        let block_ty = InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32, ValType::I32, ValType::I32],
        );
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 5 bytes from the offset is within the buffer
        block
            .local_get(offset_local)
            .i32_const(5)
            .binop(BinaryOp::I32Add)
            .local_get(end_local)
            .binop(BinaryOp::I32GtU)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0).i32_const(0).i32_const(0);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the prefix byte
        block.local_get(offset_local).load(
            memory,
            LoadKind::I32_8 {
                kind: ExtendedLoad::ZeroExtend,
            },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Check for the string-ascii prefix (0x0d)
        block
            .i32_const(TypePrefix::StringASCII as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                block_ty,
                |then| {
                    // Read the string length
                    let string_length = self.module.locals.add(ValType::I32);
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .call(self.func_by_name("stdlib.load-i32-be"))
                        .local_tee(string_length);

                    // Verify that the string length is within the
                    // string type size.
                    then.i32_const(type_length as i32)
                        .binop(BinaryOp::I32GtU)
                        .if_else(
                            None,
                            |inner| {
                                // Return none
                                inner.i32_const(0).i32_const(0).i32_const(0);
                                inner.br(block_id);
                            },
                            |_| {},
                        );

                    // Verify that the string length is within the
                    // buffer.
                    let computed_end = self.module.locals.add(ValType::I32);
                    then.local_get(string_length)
                        .local_get(offset_local)
                        .binop(BinaryOp::I32Add)
                        .i32_const(5)
                        .binop(BinaryOp::I32Add)
                        .local_tee(computed_end)
                        .local_get(end_local)
                        .binop(BinaryOp::I32GtU)
                        .if_else(
                            None,
                            |inner| {
                                // Return none
                                inner.i32_const(0).i32_const(0).i32_const(0);
                                inner.br(block_id);
                            },
                            |_| {},
                        );

                    // Push the `some` indicator onto the stack
                    then.i32_const(1);

                    // Validate the characters in the string
                    then.local_get(offset_local)
                        .i32_const(5)
                        .binop(BinaryOp::I32Add)
                        .local_get(string_length)
                        .call(self.func_by_name("stdlib.is-valid-string-ascii"))
                        .unop(UnaryOp::I32Eqz)
                        .if_else(
                            None,
                            |inner| {
                                // Return none
                                inner.i32_const(0).i32_const(0).i32_const(0);
                                inner.br(block_id);
                            },
                            |_| {},
                        );

                    // The serialized string is represented in the same
                    // way that Clarity-Wasm expects, after the type prefix
                    // and size, so just return a pointer to the corresponding
                    // part of the serialized buffer.
                    then.local_get(offset_local)
                        .i32_const(5)
                        .binop(BinaryOp::I32Add);

                    // Push the buffer length onto the stack
                    then.local_get(string_length);

                    // Increment the offset by the length of the serialized
                    // buffer.
                    then.local_get(computed_end).local_set(offset_local);
                },
                |else_| {
                    // Invalid prefix, return `none`.
                    else_.i32_const(0).i32_const(0).i32_const(0);
                },
            );

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

        Ok(())
    }

    /// Deserialize a `string-utf8` from memory using consensus serialization.
    /// Leaves an `(optional (string-utf8 n))` on the top of the data stack.
    /// See SIP-005 for details.
    ///
    /// Representation:
    ///  | 0x0e | length: 4-bytes (big-endian) | utf8-encoded string: variable length |
    fn deserialize_string_utf8(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
        offset_result: LocalId,
        string_utf8_ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let max_len: u32 = match string_utf8_ty {
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                type_length,
            ))) => type_length.into(),
            _ => {
                return Err(GeneratorError::InternalError(
                    "Shouldn't call this function on anything but string-utf8".into(),
                ))
            }
        };

        // Verify that reading 5 bytes from the offset is within the buffer
        builder
            .local_get(offset_local)
            .i32_const(5)
            .binop(BinaryOp::I32Add)
            .local_get(end_local)
            .binop(BinaryOp::I32LeU);

        // Check for the string-utf8 prefix (0x0e)
        builder
            .local_get(offset_local)
            .load(
                memory,
                LoadKind::I32_8 {
                    kind: ExtendedLoad::ZeroExtend,
                },
                MemArg {
                    align: 1,
                    offset: 0,
                },
            )
            .i32_const(TypePrefix::StringUTF8 as i32)
            .binop(BinaryOp::I32Eq);

        let return_type = InstrSeqType::new(&mut self.module.types, &[], &[ValType::I32; 3]);

        // If both previous conditions are met, we can try deserializing.
        // Otherwise, it's a failure.
        builder.binop(BinaryOp::I32And).if_else(
            return_type,
            |then| {
                // Read the string length in bytes
                let string_length = self.module.locals.add(ValType::I32);
                then.local_get(offset_local)
                    .i32_const(1)
                    .binop(BinaryOp::I32Add)
                    .call(self.func_by_name("stdlib.load-i32-be"))
                    .local_tee(string_length);

                // If the (string length <= remaining bytes), we can try deserializing,
                // otherwise it's a failure
                then.local_get(offset_local)
                    .i32_const(5)
                    .binop(BinaryOp::I32Add)
                    .local_tee(offset_local)
                    .binop(BinaryOp::I32Add)
                    .local_get(end_local)
                    .binop(BinaryOp::I32LeU);

                then.if_else(
                    return_type,
                    |then| {
                        // convert utf8 to string-utf8
                        then.local_get(offset_local)
                            .local_get(string_length)
                            .local_get(offset_result)
                            .i32_const(max_len as i32)
                            .call(self.func_by_name("stdlib.utf8-to-string-utf8"));

                        // move offset-local to the end of the utf8 representation
                        then.local_get(offset_local)
                            .local_get(string_length)
                            .binop(BinaryOp::I32Add)
                            .local_set(offset_local);

                        // move offset-result to the end of the deserialized utf8 string
                        then.local_get(offset_result)
                            .i32_const(max_len as i32 * 4)
                            .binop(BinaryOp::I32Add)
                            .local_set(offset_result);
                    },
                    |else_| {
                        else_.i32_const(0).i32_const(0).i32_const(0);
                    },
                );
            },
            |else_| {
                else_.i32_const(0).i32_const(0).i32_const(0);
            },
        );

        Ok(())
    }

    /// Deserialize a buffer in memory using the consensus serialization rules.
    /// The offset and length of the buffer are on the top of the data stack.
    /// Leaves `(some value)` on the top of the stack, or `none` if
    /// deserialization fails. It also updates `offset_local` to point to the
    /// next byte after the bytes used for deserialization. The top-level
    /// caller of this function should verify that the entire buffer was used
    /// in deserialization.
    /// See SIP-005 for deserialization details.
    pub(crate) fn deserialize_from_memory(
        &mut self,
        builder: &mut InstrSeqBuilder,
        offset_local: LocalId,
        end_local: LocalId,
        offset_result: LocalId,
        ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let memory = self.get_memory()?;

        use clarity::vm::types::signatures::TypeSignature::*;
        match ty {
            IntType | UIntType => {
                self.deserialize_integer(builder, memory, offset_local, end_local, ty == &IntType)
            }
            PrincipalType | CallableType(_) | TraitReferenceType(_) => {
                self.deserialize_principal(builder, memory, offset_local, offset_result, end_local)
            }
            ResponseType(types) => self.deserialize_response(
                builder,
                memory,
                offset_local,
                end_local,
                offset_result,
                &types.0,
                &types.1,
            ),
            BoolType => self.deserialize_bool(builder, memory, offset_local, end_local),
            OptionalType(value_ty) => self.deserialize_optional(
                builder,
                memory,
                offset_local,
                end_local,
                offset_result,
                value_ty,
            ),
            SequenceType(SequenceSubtype::ListType(list_ty)) => self.deserialize_list(
                builder,
                memory,
                offset_local,
                end_local,
                offset_result,
                list_ty,
            ),
            SequenceType(SequenceSubtype::BufferType(type_length)) => self.deserialize_buffer(
                builder,
                memory,
                offset_local,
                end_local,
                type_length.into(),
            ),
            SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(type_length))) => self
                .deserialize_string_ascii(
                    builder,
                    memory,
                    offset_local,
                    end_local,
                    type_length.into(),
                ),
            utf8 @ SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => self
                .deserialize_string_utf8(
                    builder,
                    memory,
                    offset_local,
                    end_local,
                    offset_result,
                    utf8,
                ),
            TupleType(tuple_ty) => self.deserialize_tuple(
                builder,
                memory,
                offset_local,
                end_local,
                offset_result,
                tuple_ty,
            ),
            NoType => unreachable!("NoType should not be deserialized"),
            ListUnionType(_) => unreachable!("ListUnionType should not be deserialized"),
        }
    }
}
