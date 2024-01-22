use clarity::vm::clarity_wasm::{get_type_size, PRINCIPAL_BYTES, STANDARD_PRINCIPAL_BYTES};
use clarity::vm::types::serialization::TypePrefix;
use clarity::vm::types::{
    ListTypeData, SequenceSubtype, StringSubtype, TupleTypeSignature, TypeSignature,
};
use walrus::ir::{BinaryOp, IfElse, InstrSeqType, Loop, MemArg, StoreKind};
use walrus::{InstrSeqBuilder, LocalId, MemoryId, ValType};

use crate::wasm_generator::{clar2wasm_ty, GeneratorError, WasmGenerator};

impl WasmGenerator {
    /// Serialize an integer (`int` or `uint`) to memory using consensus
    /// serialization. Leaves the length of the data written on the top of the
    /// data stack. See SIP-005 for details.
    ///
    /// Representation:
    ///   Int:
    ///     | 0x00 | value: 16-bytes (big-endian) |
    ///   UInt:
    ///     | 0x01 | value: 16-bytes (big-endian) |
    fn serialize_integer(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
        signed: bool,
    ) -> Result<(), GeneratorError> {
        let mut written = 0;

        // Data stack: TOP | High | Low |
        // Save the high/low to locals.
        let high = self.module.locals.add(ValType::I64);
        let low = self.module.locals.add(ValType::I64);
        builder.local_set(high).local_set(low);

        // Create a local for the write pointer by adjusting the
        // offset local by the offset amount.
        let write_ptr = self.module.locals.add(ValType::I32);
        if offset > 0 {
            builder
                .local_get(offset_local)
                .i32_const(offset as i32)
                .binop(BinaryOp::I32Add)
                .local_tee(write_ptr);
        } else {
            builder.local_get(offset_local).local_tee(write_ptr);
        }

        // Write the type prefix first
        let prefix = if signed {
            TypePrefix::Int
        } else {
            TypePrefix::UInt
        };
        builder.i32_const(prefix as i32).store(
            memory,
            StoreKind::I32_8 { atomic: false },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Adjust the write pointer
        builder
            .local_get(write_ptr)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);
        written += 1;

        // Serialize the high to memory.
        builder
            .local_get(high)
            .call(self.func_by_name("stdlib.store-i64-be"));

        // Adjust the write pointer
        builder
            .local_get(write_ptr)
            .i32_const(8)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);
        written += 8;

        // Adjust the offset by 8, then serialize the low to memory.
        builder
            .local_get(low)
            .call(self.func_by_name("stdlib.store-i64-be"));
        written += 8;

        // Push the written length onto the data stack
        builder.i32_const(written);

        Ok(())
    }

    /// Serialize a `principal` to memory using consensus serialization. Leaves
    /// the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///   Standard:
    ///    | 0x05 | version: 1 byte | public key(s)' hash160: 20-bytes |
    ///   Contract:
    ///    | 0x06 | version: 1 byte | public key(s)' hash160: 20-bytes
    ///      | contract name length: 1 byte | contract name: variable length |
    fn serialize_principal(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Length | Offset |
        // Save the offset/length to locals.
        let poffset = self.module.locals.add(ValType::I32);
        let plength = self.module.locals.add(ValType::I32);
        builder.local_set(plength).local_set(poffset);

        // Create a local for the write pointer by adjusting the
        // offset local by the offset amount.
        let write_ptr = self.module.locals.add(ValType::I32);
        if offset > 0 {
            builder
                .local_get(offset_local)
                .i32_const(offset as i32)
                .binop(BinaryOp::I32Add)
                .local_tee(write_ptr);
        } else {
            builder.local_get(offset_local).local_tee(write_ptr);
        }

        // Copy the standard principal part to the buffer, offset by 1
        // byte for the type prefix, which we will write next, so that
        // we don't need two branches.
        builder
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .local_get(poffset)
            .i32_const(PRINCIPAL_BYTES as i32)
            .memory_copy(memory, memory);

        // If `plength` is greater than STANDARD_PRINCIPAL_BYTES, then
        // this is a contract principal, else, it's a standard
        // principal.
        builder
            .local_get(plength)
            .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
            .binop(BinaryOp::I32GtS)
            .if_else(
                InstrSeqType::new(&mut self.module.types, &[], &[ValType::I32]),
                |then| {
                    // Write the total length of the contract to the buffer
                    then
                        // Compute the destination offset
                        .local_get(write_ptr)
                        .i32_const(PRINCIPAL_BYTES as i32 + 1)
                        .binop(BinaryOp::I32Add)
                        // Compute the length
                        .local_get(plength)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Sub)
                        // Write the length
                        .store(
                            memory,
                            StoreKind::I32_8 { atomic: false },
                            MemArg {
                                align: 1,
                                offset: 0,
                            },
                        );

                    // Copy the contract name to the buffer
                    then
                        // Compute the destination offset
                        .local_get(write_ptr)
                        .i32_const(PRINCIPAL_BYTES as i32 + 2)
                        .binop(BinaryOp::I32Add)
                        // Compute the source offset
                        .local_get(poffset)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Add)
                        // Compute the length
                        .local_get(plength)
                        .i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Sub)
                        // Copy the data
                        .memory_copy(memory, memory);

                    // Push the total length written onto the data stack.
                    // It is the same as plength, plus 1 (the type prefix).
                    then.local_get(plength).i32_const(1).binop(BinaryOp::I32Add);

                    // Push the type prefix for a contract principal
                    then.local_get(write_ptr)
                        .i32_const(TypePrefix::PrincipalContract as i32)
                        .store(
                            memory,
                            StoreKind::I32_8 { atomic: false },
                            MemArg {
                                align: 1,
                                offset: 0,
                            },
                        );
                },
                |else_| {
                    // Push the total length written onto the data stack.
                    else_.i32_const(PRINCIPAL_BYTES as i32 + 1);

                    // Store the type prefix for a standard principal
                    else_
                        .local_get(write_ptr)
                        .i32_const(TypePrefix::PrincipalStandard as i32)
                        .store(
                            memory,
                            StoreKind::I32_8 { atomic: false },
                            MemArg {
                                align: 1,
                                offset: 0,
                            },
                        );
                },
            );
        Ok(())
    }

    /// Serialize a `response` to memory using consensus serialization. Leaves
    /// the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///   Ok:
    ///    | 0x07 | serialized ok value |
    ///   Err:
    ///    | 0x08 | serialized err value |
    fn serialize_response(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
        types: &(TypeSignature, TypeSignature),
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Err Value | Ok Value | Indicator |
        // Save the error values to locals
        let err_locals = self.save_to_locals(builder, &types.1, true);

        // Save the ok values to locals
        let ok_locals = self.save_to_locals(builder, &types.0, true);

        // Create a block for the ok case
        let mut ok_block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32],
        ));
        let ok_block_id = ok_block.id();

        // Write the type prefix to memory
        ok_block
            .local_get(offset_local)
            .i32_const(TypePrefix::ResponseOk as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Push the ok value back onto the stack
        for local in ok_locals.iter() {
            ok_block.local_get(*local);
        }

        // Now serialize the ok value to memory
        self.serialize_to_memory(&mut ok_block, offset_local, offset + 1, &types.0)?;

        // Create a block for the err case
        let mut err_block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32],
        ));
        let err_block_id = err_block.id();

        // Write the type prefix to memory
        err_block
            .local_get(offset_local)
            .i32_const(TypePrefix::ResponseErr as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Push the err value back onto the stack
        for local in err_locals.iter() {
            err_block.local_get(*local);
        }

        // Now serialize the err value to memory
        self.serialize_to_memory(&mut err_block, offset_local, offset + 1, &types.1)?;

        // The top of the stack is currently the indicator, which is
        // `1` for `ok` and `0` for err.
        builder.instr(IfElse {
            consequent: ok_block_id,
            alternative: err_block_id,
        });

        // Increment the amount written by 1 for the indicator
        builder.i32_const(1).binop(BinaryOp::I32Add);

        Ok(())
    }

    /// Serialize a `bool` to memory using consensus serialization. Leaves the
    /// length of the data written on the top of the data stack. See SIP-005
    /// for details.
    /// Representation:
    ///   True:
    ///    | 0x03 |
    ///   False:
    ///    | 0x04 |
    fn serialize_bool(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
    ) -> Result<(), GeneratorError> {
        // Save the bool to a local
        let local = self.module.locals.add(ValType::I32);
        builder.local_set(local);

        // Load the location to write to
        builder.local_get(offset_local);

        // Select the appropriate type prefix
        builder
            .i32_const(TypePrefix::BoolTrue as i32)
            .i32_const(TypePrefix::BoolFalse as i32)
            .local_get(local)
            .select(Some(ValType::I32));

        // Write the type prefix to memory
        builder.store(
            memory,
            StoreKind::I32_8 { atomic: false },
            MemArg { align: 1, offset },
        );

        // Push the amount written to the data stack
        builder.i32_const(1);

        Ok(())
    }

    /// Serialize an `optional` to memory using consensus serialization. Leaves
    /// the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///   None:
    ///    | 0x09 |
    ///   Some:
    ///    | 0x0a | serialized value |
    fn serialize_optional(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
        value_ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Value | Indicator |
        // Save the values to locals
        let locals = self.save_to_locals(builder, value_ty, true);

        // Create a block for the some case
        let mut some_block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32],
        ));
        let some_block_id = some_block.id();

        // Write the type prefix to memory
        some_block
            .local_get(offset_local)
            .i32_const(TypePrefix::OptionalSome as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Push the some value back onto the stack
        for local in locals.iter() {
            some_block.local_get(*local);
        }

        // Now serialize the value to memory
        self.serialize_to_memory(&mut some_block, offset_local, offset + 1, value_ty)?;

        // Increment the amount written by 1 for the indicator
        some_block.i32_const(1).binop(BinaryOp::I32Add);

        // Create a block for the none case
        let mut none_block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            &[ValType::I32],
        ));
        let none_block_id = none_block.id();

        // Write the type prefix to memory
        none_block
            .local_get(offset_local)
            .i32_const(TypePrefix::OptionalNone as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        none_block.i32_const(1);

        // The top of the stack is currently the indicator, which is
        // `1` for `some` and `0` for none.
        builder.instr(IfElse {
            consequent: some_block_id,
            alternative: none_block_id,
        });

        Ok(())
    }

    /// Serialize a `list` to memory using consensus serialization. Leaves
    /// the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///    | 0x0b | number of elements: 4-bytes (big-endian)
    ///         | serialized representation of element 0
    ///         | serialized representation of element 1
    ///         | ...
    fn serialize_list(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
        list_ty: &ListTypeData,
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Length | Offset |
        let write_ptr = self.module.locals.add(ValType::I32);
        let read_ptr = self.module.locals.add(ValType::I32);
        let bytes_length = self.module.locals.add(ValType::I32);

        // Write the type prefix to memory
        builder
            .local_get(offset_local)
            .i32_const(TypePrefix::List as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Save the length of the list to a local
        builder.local_set(bytes_length);
        builder.local_set(read_ptr);

        // if bytes_length is zero, we can simply add 0_i32 to the serialized buffer,
        // otherwise, we'll loop through elements and serialize them one by one.

        let size_zero_id = {
            let mut size_zero = builder.dangling_instr_seq(ValType::I32);

            size_zero.local_get(offset_local).i32_const(0).store(
                memory,
                StoreKind::I32 { atomic: false },
                MemArg {
                    align: 1,
                    offset: offset + 1,
                },
            );

            size_zero.i32_const(5);
            size_zero.id()
        };

        let size_non_zero_id = {
            let mut size_non_zero = builder.dangling_instr_seq(ValType::I32);

            let element_ty = list_ty.get_list_item_type();
            let element_size = get_type_size(element_ty);

            // set write pointer
            size_non_zero
                .local_get(offset_local)
                .i32_const(offset as i32 + 1)
                .binop(BinaryOp::I32Add)
                .local_tee(write_ptr);

            // compute size of list and store it as big-endian i32
            size_non_zero
                .local_get(bytes_length)
                .i32_const(element_size)
                .binop(BinaryOp::I32DivU);
            size_non_zero.call(self.func_by_name("stdlib.store-i32-be"));

            // Adjust the write pointer
            size_non_zero
                .local_get(write_ptr)
                .i32_const(4)
                .binop(BinaryOp::I32Add)
                .local_set(write_ptr);

            // Loop through elements and serialize
            let loop_id = {
                let mut loop_ = size_non_zero.dangling_instr_seq(None);
                let loop_id = loop_.id();

                self.read_from_memory(&mut loop_, read_ptr, 0, element_ty)?;

                self.serialize_to_memory(&mut loop_, write_ptr, 0, element_ty)?;

                // Adjust pointers (for write_ptr, adjustment is on the stack)
                loop_
                    .local_get(write_ptr)
                    .binop(BinaryOp::I32Add)
                    .local_set(write_ptr);
                loop_
                    .local_get(read_ptr)
                    .i32_const(element_size)
                    .binop(BinaryOp::I32Add)
                    .local_set(read_ptr);

                // we loop while there are bytes to read in the list
                loop_
                    .local_get(bytes_length)
                    .i32_const(element_size)
                    .binop(BinaryOp::I32Sub)
                    .local_tee(bytes_length)
                    .br_if(loop_id);

                loop_id
            };

            size_non_zero.instr(Loop { seq: loop_id });

            // Push the amount written to the data stack
            size_non_zero
                .local_get(write_ptr)
                .local_get(offset_local)
                .i32_const(offset as i32)
                .binop(BinaryOp::I32Add)
                .binop(BinaryOp::I32Sub);

            size_non_zero.id()
        };

        builder
            .local_get(bytes_length)
            .unop(walrus::ir::UnaryOp::I32Eqz)
            .instr(IfElse {
                consequent: size_zero_id,
                alternative: size_non_zero_id,
            });

        Ok(())
    }

    /// Serialize a `buffer` to memory using consensus serialization. Leaves
    /// the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///  | 0x02 | length: 4-bytes (big-endian) | data: variable length |
    fn serialize_buffer(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Length | Offset |
        let write_ptr = self.module.locals.add(ValType::I32);
        let read_ptr = self.module.locals.add(ValType::I32);
        let length = self.module.locals.add(ValType::I32);

        // Save the length and offset to locals
        builder.local_set(length).local_set(read_ptr);

        // Write the type prefix first
        builder
            .local_get(offset_local)
            .i32_const(TypePrefix::Buffer as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Create a local for the write pointer by adjusting the
        // offset local by the offset amount + 1 for the prefix.
        builder
            .local_get(offset_local)
            .i32_const(offset as i32 + 1)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);

        // Serialize the length to memory (big endian)
        builder
            .local_get(length)
            .call(self.func_by_name("stdlib.store-i32-be"));

        // Adjust the write pointer by 4
        builder
            .local_get(write_ptr)
            .i32_const(4)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);

        // Copy the buffer
        builder
            .local_get(read_ptr)
            .local_get(length)
            .memory_copy(memory, memory);

        // Push the length written to the data stack:
        //  length    +    1    +    4
        //      type prefix^         ^length
        builder
            .local_get(length)
            .i32_const(5)
            .binop(BinaryOp::I32Add);

        Ok(())
    }

    /// Serialize a `string-ascii` to memory using consensus serialization.
    /// Leaves the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///  | 0x0d | length: 4-bytes (big-endian) | ascii-encoded string: variable length |
    fn serialize_string_ascii(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Length | Offset |
        let write_ptr = self.module.locals.add(ValType::I32);
        let read_ptr = self.module.locals.add(ValType::I32);
        let length = self.module.locals.add(ValType::I32);

        // Save the length and offset to locals
        builder.local_set(length).local_set(read_ptr);

        // Write the type prefix first
        builder
            .local_get(offset_local)
            .i32_const(TypePrefix::StringASCII as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Create a local for the write pointer by adjusting the
        // offset local by the offset amount + 1 for the prefix.
        builder
            .local_get(offset_local)
            .i32_const(offset as i32 + 1)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);

        // Serialize the length to memory (big endian)
        builder
            .local_get(length)
            .call(self.func_by_name("stdlib.store-i32-be"));

        // Adjust the write pointer by 4
        builder
            .local_get(write_ptr)
            .i32_const(4)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);

        // Copy the string
        builder
            .local_get(read_ptr)
            .local_get(length)
            .memory_copy(memory, memory);

        // Push the length written to the data stack:
        //  length    +    1    +    4
        //      type prefix^         ^length
        builder
            .local_get(length)
            .i32_const(5)
            .binop(BinaryOp::I32Add);

        Ok(())
    }

    /// Serialize a `string-utf8` to memory using consensus serialization.
    /// Leaves the length of the data written on the top of the data stack. See
    /// SIP-005 for details.
    /// Representation:
    ///  | 0x0e | length: 4-bytes (big-endian) | utf8-encoded string: variable length |
    fn serialize_string_utf8(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
    ) -> Result<(), GeneratorError> {
        // Data stack: TOP | Length | Offset |
        let write_ptr = self.module.locals.add(ValType::I32);
        let read_ptr = self.module.locals.add(ValType::I32);
        let length = self.module.locals.add(ValType::I32);
        let utf8_length = self.module.locals.add(ValType::I32);

        // Save the length and offset to locals
        builder.local_set(length).local_set(read_ptr);

        // Write the type prefix first
        builder
            .local_get(offset_local)
            .i32_const(TypePrefix::StringUTF8 as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Create a local for the write pointer by adjusting the
        // offset local by the offset amount + 1 (prefix) + 4 (length).
        builder
            .local_get(offset_local)
            .i32_const(offset as i32 + 5)
            .binop(BinaryOp::I32Add)
            .local_set(write_ptr);

        // Push the offset, length, and output-offset to the data stack
        builder
            .local_get(read_ptr)
            .local_get(length)
            .local_get(write_ptr);

        // Call scalar to utf8 conversion function
        builder
            .call(self.func_by_name("stdlib.convert-scalars-to-utf8"))
            .local_tee(utf8_length);

        // Serialize the length to memory (big endian)
        builder
            .local_get(offset_local)
            .i32_const(offset as i32 + 1)
            .binop(BinaryOp::I32Add)
            .local_get(utf8_length)
            .call(self.func_by_name("stdlib.store-i32-be"));

        // Push the length written to the data stack, the length of the serialized string is already on the stack
        //  length    +    1    +    4
        //      type prefix^         ^length
        builder.i32_const(5).binop(BinaryOp::I32Add);

        Ok(())
    }

    /// Serialize a `tuple` to memory using consensus serialization. Leaves the
    /// length of the data written on the top of the data stack. See SIP-005
    /// for details.
    /// Representation:
    ///  | 0x0c | number of keys: 4-bytes (big-endian)
    ///    | key 0 length: 1-byte | key 0: variable length | serialized value 0
    ///    ...
    ///    | key N length: 1-byte | key N: variable length | serialized value N
    fn serialize_tuple(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        offset: u32,
        ty: &TypeSignature,
        tuple_ty: &TupleTypeSignature,
    ) -> Result<(), GeneratorError> {
        // In Wasm, tuples are represented as a sequence of values
        // concatenated together. The keys are not included in the Wasm
        // representation of a tuple, so we get the keys from the type
        // and the values from the data stack.

        let write_ptr = self.module.locals.add(ValType::I32);

        // First, save the values to locals, so that we can get them in
        // the correct order.
        let mut locals = self.save_to_locals(builder, ty, false);

        // Now write the type prefix to memory
        builder
            .local_get(offset_local)
            .i32_const(TypePrefix::Tuple as i32)
            .store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg { align: 1, offset },
            );

        // Initialize the write pointer
        builder
            .local_get(offset_local)
            .i32_const(offset as i32 + 1)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);

        // Serialize the length of the data map to memory (big endian)
        builder
            .i32_const(tuple_ty.get_type_map().len() as i32)
            .call(self.func_by_name("stdlib.store-i32-be"));

        // Adjust the write pointer by 4
        builder
            .local_get(write_ptr)
            .i32_const(4)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);

        // Now serialize the keys/values to memory
        for (key, value_ty) in tuple_ty.get_type_map() {
            // Serialize the key length
            builder.i32_const(key.len() as i32).store(
                memory,
                StoreKind::I32_8 { atomic: false },
                MemArg {
                    align: 1,
                    offset: 0,
                },
            );

            // Adjust the write pointer
            builder
                .local_get(write_ptr)
                .i32_const(1)
                .binop(BinaryOp::I32Add)
                .local_tee(write_ptr);

            // Serialize the key name
            let (offset, length) = self.add_string_literal(key);
            builder
                .i32_const(offset as i32)
                .i32_const(length as i32)
                .memory_copy(memory, memory);

            // Adjust the write pointer
            builder
                .local_get(write_ptr)
                .i32_const(length as i32)
                .binop(BinaryOp::I32Add)
                .local_set(write_ptr);

            // Push the next value back onto the stack
            let wasm_types = clar2wasm_ty(value_ty);
            for _ in 0..wasm_types.len() {
                builder.local_get(
                    locals.pop().ok_or_else(|| {
                        GeneratorError::InternalError("invalid tuple value".into())
                    })?,
                );
            }

            // Serialize the value
            self.serialize_to_memory(builder, write_ptr, 0, value_ty)?;

            // Adjust the write pointer by the length left on the stack
            builder
                .local_get(write_ptr)
                .binop(BinaryOp::I32Add)
                .local_tee(write_ptr);
        }

        // Push the amount written to the data stack
        builder
            .local_get(offset_local)
            .i32_const(offset as i32)
            .binop(BinaryOp::I32Add)
            .binop(BinaryOp::I32Sub);

        Ok(())
    }

    /// Serialize the value of type `ty` on the top of the data stack using
    /// consensus serialization. Leaves the length of the data written on the
    /// top of the data stack. See SIP-005 for details.
    pub(crate) fn serialize_to_memory(
        &mut self,
        builder: &mut InstrSeqBuilder,
        offset_local: LocalId,
        offset: u32,
        ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let memory = self.get_memory();

        use clarity::vm::types::signatures::TypeSignature::*;
        match ty {
            IntType | UIntType => {
                self.serialize_integer(builder, memory, offset_local, offset, ty == &IntType)
            }
            PrincipalType | CallableType(_) | TraitReferenceType(_) => {
                self.serialize_principal(builder, memory, offset_local, offset)
            }
            ResponseType(types) => {
                self.serialize_response(builder, memory, offset_local, offset, types)
            }
            BoolType => self.serialize_bool(builder, memory, offset_local, offset),
            OptionalType(value_ty) => {
                self.serialize_optional(builder, memory, offset_local, offset, value_ty)
            }
            SequenceType(SequenceSubtype::ListType(list_ty)) => {
                self.serialize_list(builder, memory, offset_local, offset, list_ty)
            }
            SequenceType(SequenceSubtype::BufferType(_)) => {
                self.serialize_buffer(builder, memory, offset_local, offset)
            }
            SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                self.serialize_string_ascii(builder, memory, offset_local, offset)
            }
            SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                self.serialize_string_utf8(builder, memory, offset_local, offset)
            }
            TupleType(tuple_ty) => {
                self.serialize_tuple(builder, memory, offset_local, offset, ty, tuple_ty)
            }
            NoType => {
                // This type should not actually be serialized. It is
                // reporesented as an `i32` value of `0`, so we can leave
                // that on top of the stack indicating 0 bytes written.
                Ok(())
            }
            ListUnionType(_) => unreachable!("ListUnionType should not be serialized"),
        }
    }
}
