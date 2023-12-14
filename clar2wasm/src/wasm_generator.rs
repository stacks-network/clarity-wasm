use std::borrow::BorrowMut;
use std::collections::HashMap;

use clarity::vm::analysis::ContractAnalysis;
use clarity::vm::clarity_wasm::{
    get_type_in_memory_size, get_type_size, is_in_memory_type, PRINCIPAL_BYTES,
    STANDARD_PRINCIPAL_BYTES,
};
use clarity::vm::diagnostic::DiagnosableError;
use clarity::vm::types::serialization::TypePrefix;
use clarity::vm::types::{
    CharType, FunctionType, ListTypeData, PrincipalData, SequenceData, SequenceSubtype,
    StringSubtype, TupleTypeSignature, TypeSignature,
};
use clarity::vm::variables::NativeVariables;
use clarity::vm::{ClarityName, SymbolicExpression, SymbolicExpressionType};
use walrus::ir::{
    BinaryOp, Const, ExtendedLoad, IfElse, InstrSeqId, InstrSeqType, LoadKind, Loop, MemArg,
    StoreKind, UnaryOp,
};
use walrus::{
    ActiveData, DataKind, FunctionBuilder, FunctionId, GlobalId, InstrSeqBuilder, LocalId,
    MemoryId, Module, ValType,
};

use crate::words;

/// First free position after data directly defined in standard.wat
pub const END_OF_STANDARD_DATA: u32 = 1352;

/// WasmGenerator is a Clarity AST visitor that generates a WebAssembly module
/// as it traverses the AST.
pub struct WasmGenerator {
    /// The contract analysis, which contains the expressions and type
    /// information for the contract.
    pub(crate) contract_analysis: ContractAnalysis,
    /// The WebAssembly module that is being generated.
    pub(crate) module: Module,
    /// Offset of the end of the literal memory.
    pub(crate) literal_memory_end: u32,
    /// Global ID of the stack pointer.
    pub(crate) stack_pointer: GlobalId,
    /// Map strings saved in the literal memory to their offset.
    pub(crate) literal_memory_offset: HashMap<LiteralMemoryEntry, u32>,
    /// Map constants to an offset in the literal memory.
    pub(crate) constants: HashMap<String, u32>,
    /// The current function body block, used for early exit
    early_return_block_id: Option<InstrSeqId>,
    /// The return type of the current function.
    pub(crate) return_type: Option<TypeSignature>,

    /// The locals for the current function.
    pub(crate) bindings: HashMap<String, Vec<LocalId>>,
    /// Size of the current function's stack frame.
    frame_size: i32,
}

#[derive(Hash, Eq, PartialEq)]
pub enum LiteralMemoryEntry {
    Ascii(String),
    Utf8(String),
}

#[derive(Debug)]
pub enum GeneratorError {
    NotImplemented,
    InternalError(String),
    TypeError(String),
}

pub enum FunctionKind {
    Public,
    Private,
    ReadOnly,
}

impl DiagnosableError for GeneratorError {
    fn message(&self) -> String {
        match self {
            GeneratorError::NotImplemented => "Not implemented".to_string(),
            GeneratorError::InternalError(msg) => format!("Internal error: {}", msg),
            GeneratorError::TypeError(msg) => format!("Type error: {}", msg),
        }
    }

    fn suggestion(&self) -> Option<String> {
        None
    }
}

pub trait ArgumentsExt {
    fn get_expr(&self, n: usize) -> Result<&SymbolicExpression, GeneratorError>;
    fn get_name(&self, n: usize) -> Result<&ClarityName, GeneratorError>;
    fn get_list(&self, n: usize) -> Result<&[SymbolicExpression], GeneratorError>;
}

impl ArgumentsExt for &[SymbolicExpression] {
    fn get_expr(&self, n: usize) -> Result<&SymbolicExpression, GeneratorError> {
        self.get(n).ok_or(GeneratorError::InternalError(format!(
            "{self:?} does not have an argument of index {n}"
        )))
    }

    fn get_name(&self, n: usize) -> Result<&ClarityName, GeneratorError> {
        self.get_expr(n)?
            .match_atom()
            .ok_or(GeneratorError::InternalError(format!(
                "{self:?} does not have a name at argument index {n}"
            )))
    }

    fn get_list(&self, n: usize) -> Result<&[SymbolicExpression], GeneratorError> {
        self.get_expr(n)?
            .match_list()
            .ok_or(GeneratorError::InternalError(format!(
                "{self:?} does not have a list at argument index {n}"
            )))
    }
}

/// Push a placeholder value for Wasm type `ty` onto the data stack.
pub(crate) fn add_placeholder_for_type(builder: &mut InstrSeqBuilder, ty: ValType) {
    match ty {
        ValType::I32 => builder.i32_const(0),
        ValType::I64 => builder.i64_const(0),
        ValType::F32 => builder.f32_const(0.0),
        ValType::F64 => builder.f64_const(0.0),
        ValType::V128 => unimplemented!("V128"),
        ValType::Externref => unimplemented!("Externref"),
        ValType::Funcref => unimplemented!("Funcref"),
    };
}

/// Push a placeholder value for Clarity type `ty` onto the data stack.
pub(crate) fn add_placeholder_for_clarity_type(builder: &mut InstrSeqBuilder, ty: &TypeSignature) {
    let wasm_types = clar2wasm_ty(ty);
    for wasm_type in wasm_types.iter() {
        add_placeholder_for_type(builder, *wasm_type);
    }
}

impl WasmGenerator {
    pub fn new(contract_analysis: ContractAnalysis) -> WasmGenerator {
        let standard_lib_wasm: &[u8] = include_bytes!("standard/standard.wasm");
        let module =
            Module::from_buffer(standard_lib_wasm).expect("failed to load standard library");

        // Get the stack-pointer global ID
        let stack_pointer_name = "stack-pointer";
        let global_id = module
            .globals
            .iter()
            .find(|global| {
                global
                    .name
                    .as_ref()
                    .map_or(false, |name| name == stack_pointer_name)
            })
            .map(|global| global.id())
            .expect("Expected to find a global named $stack-pointer");

        WasmGenerator {
            contract_analysis,
            module,
            literal_memory_end: END_OF_STANDARD_DATA,
            stack_pointer: global_id,
            literal_memory_offset: HashMap::new(),
            constants: HashMap::new(),
            bindings: HashMap::new(),
            early_return_block_id: None,
            return_type: None,
            frame_size: 0,
        }
    }

    pub fn generate(mut self) -> Result<Module, GeneratorError> {
        let expressions = std::mem::take(&mut self.contract_analysis.expressions);
        // println!("{:?}", expressions);

        // Get the type of the last top-level expression
        let return_ty = expressions
            .last()
            .and_then(|last_expr| self.get_expr_type(last_expr))
            .map_or_else(Vec::new, clar2wasm_ty);

        let mut current_function = FunctionBuilder::new(&mut self.module.types, &[], &return_ty);

        if !expressions.is_empty() {
            self.traverse_statement_list(&mut current_function.func_body(), &expressions)?;
        }

        self.contract_analysis.expressions = expressions;

        let top_level = current_function.finish(vec![], &mut self.module.funcs);
        self.module.exports.add(".top-level", top_level);

        // Update the initial value of the stack-pointer to point beyond the
        // literal memory.
        self.module.globals.get_mut(self.stack_pointer).kind = walrus::GlobalKind::Local(
            walrus::InitExpr::Value(walrus::ir::Value::I32(self.literal_memory_end as i32)),
        );

        Ok(self.module)
    }

    pub fn get_memory(&self) -> MemoryId {
        self.module
            .memories
            .iter()
            .next()
            .expect("no memory found")
            .id()
    }

    pub fn traverse_expr(
        &mut self,
        builder: &mut InstrSeqBuilder,
        expr: &SymbolicExpression,
    ) -> Result<(), GeneratorError> {
        match &expr.expr {
            SymbolicExpressionType::Atom(name) => self.visit_atom(builder, expr, name),
            SymbolicExpressionType::List(exprs) => self.traverse_list(builder, expr, exprs),
            SymbolicExpressionType::LiteralValue(value) => {
                self.visit_literal_value(builder, expr, value)
            }
            _ => Ok(()),
        }
    }

    fn traverse_list(
        &mut self,
        builder: &mut InstrSeqBuilder,
        expr: &SymbolicExpression,
        list: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        match list.split_first() {
            Some((
                SymbolicExpression {
                    expr: SymbolicExpressionType::Atom(function_name),
                    ..
                },
                args,
            )) => {
                if let Some(word) = words::lookup(function_name) {
                    word.traverse(self, builder, expr, args)?;
                } else {
                    self.traverse_call_user_defined(builder, expr, function_name, args)?;
                }
            }
            _ => todo!(),
        }
        Ok(())
    }

    pub fn traverse_define_function(
        &mut self,
        builder: &mut InstrSeqBuilder,
        name: &ClarityName,
        body: &SymbolicExpression,
        kind: FunctionKind,
    ) -> Result<FunctionId, GeneratorError> {
        let opt_function_type = match kind {
            FunctionKind::ReadOnly => {
                builder.i32_const(0);
                self.contract_analysis
                    .get_read_only_function_type(name.as_str())
            }
            FunctionKind::Public => {
                builder.i32_const(1);
                self.contract_analysis
                    .get_public_function_type(name.as_str())
            }
            FunctionKind::Private => {
                builder.i32_const(2);
                self.contract_analysis.get_private_function(name.as_str())
            }
        };
        let function_type = if let Some(FunctionType::Fixed(fixed)) = opt_function_type {
            fixed.clone()
        } else {
            return Err(GeneratorError::InternalError(match opt_function_type {
                Some(_) => "expected fixed function type".to_string(),
                None => format!("unable to find function type for {}", name.as_str()),
            }));
        };

        self.return_type = Some(function_type.returns.clone());

        // Call the host interface to save this function
        // Arguments are kind (already pushed) and name (offset, length)
        let (id_offset, id_length) = self.add_string_literal(name);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Call the host interface function, `define_function`
        builder.call(
            self.module
                .funcs
                .by_name("stdlib.define_function")
                .expect("define_function not found"),
        );

        let mut bindings = HashMap::new();

        // Setup the parameters
        let mut param_locals = Vec::new();
        let mut params_types = Vec::new();
        for param in function_type.args.iter() {
            let param_types = clar2wasm_ty(&param.signature);
            let mut plocals = Vec::with_capacity(param_types.len());
            for ty in param_types {
                let local = self.module.locals.add(ty);
                param_locals.push(local);
                plocals.push(local);
                params_types.push(ty);
            }
            bindings.insert(param.name.to_string(), plocals.clone());
        }

        let results_types = clar2wasm_ty(&function_type.returns);
        let mut func_builder = FunctionBuilder::new(
            &mut self.module.types,
            params_types.as_slice(),
            results_types.as_slice(),
        );
        func_builder.name(name.as_str().to_string());
        let mut func_body = func_builder.func_body();

        // Function prelude
        // Save the frame pointer in a local variable.
        let frame_pointer = self.module.locals.add(ValType::I32);
        func_body
            .global_get(self.stack_pointer)
            .local_set(frame_pointer);

        // Setup the locals map for this function, saving the top-level map to
        // restore after.
        let top_level_locals = std::mem::replace(&mut self.bindings, bindings);

        let mut block = func_body.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            results_types.as_slice(),
        ));
        let block_id = block.id();

        self.early_return_block_id = Some(block_id);

        // Traverse the body of the function
        self.set_expr_type(body, function_type.returns.clone());
        self.traverse_expr(&mut block, body)?;

        // TODO: We need to ensure that all exits from the function go through
        // the postlude. Maybe put the body in a block, and then have any exits
        // from the block go to the postlude with a `br` instruction?

        // Insert the function body block into the function
        func_body.instr(walrus::ir::Block { seq: block_id });

        // Function postlude
        // Restore the initial stack pointer.
        func_body
            .local_get(frame_pointer)
            .global_set(self.stack_pointer);

        // Restore the top-level locals map.
        self.bindings = top_level_locals;

        // Reset the return type and early block to None
        self.return_type = None;
        self.early_return_block_id = None;

        Ok(func_builder.finish(param_locals, &mut self.module.funcs))
    }

    pub fn return_early(&self, builder: &mut InstrSeqBuilder) -> Result<(), GeneratorError> {
        if let Some(block_id) = self.early_return_block_id {
            builder.instr(walrus::ir::Br { block: block_id });
        } else {
            // This must be from a top-leve statement, so it should cause a runtime error
            builder.i32_const(7).call(
                self.module
                    .funcs
                    .by_name("stdlib.runtime-error")
                    .expect("stdlib.runtime-error not found"),
            );
            builder.unreachable();
        }

        Ok(())
    }

    /// Gets the result type of the given `SymbolicExpression`.
    pub fn get_expr_type(&self, expr: &SymbolicExpression) -> Option<&TypeSignature> {
        self.contract_analysis
            .type_map
            .as_ref()
            .expect("type-checker must be called before Wasm generation")
            .get_type(expr)
    }

    /// Sets the result type of the given `SymbolicExpression`. This is
    /// necessary to overcome some weaknesses in the type-checker and
    /// hopefully can be removed in the future.
    pub fn set_expr_type(&mut self, expr: &SymbolicExpression, ty: TypeSignature) {
        // Safely ignore the error because we know this type has already been set.
        let _ = self
            .contract_analysis
            .type_map
            .as_mut()
            .expect("type-checker must be called before Wasm generation")
            .set_type(expr, ty);
    }

    /// Adds a new string literal into the memory, and returns the offset and length.
    pub(crate) fn add_clarity_string_literal(&mut self, s: &CharType) -> (u32, u32) {
        // If this string has already been saved in the literal memory,
        // just return the offset and length.
        let (data, entry) = match s {
            CharType::ASCII(s) => {
                let entry = LiteralMemoryEntry::Ascii(s.to_string());
                if let Some(offset) = self.literal_memory_offset.get(&entry) {
                    return (*offset, s.data.len() as u32);
                }
                (s.data.clone(), entry)
            }
            CharType::UTF8(u) => {
                let data_str = String::from_utf8(u.data.iter().flatten().cloned().collect())
                    .expect("Invalid UTF-8 sequence");
                let entry = LiteralMemoryEntry::Utf8(data_str.clone());
                if let Some(offset) = self.literal_memory_offset.get(&entry) {
                    return (*offset, u.data.len() as u32 * 4);
                }
                // Convert the string into 4-byte big-endian unicode scalar values.
                let data = data_str
                    .chars()
                    .flat_map(|c| (c as u32).to_be_bytes())
                    .collect();
                (data, entry)
            }
        };
        let memory = self.module.memories.iter().next().expect("no memory found");
        let offset = self.literal_memory_end;
        let len = data.len() as u32;
        self.module.data.add(
            DataKind::Active(ActiveData {
                memory: memory.id(),
                location: walrus::ActiveDataLocation::Absolute(offset),
            }),
            data,
        );
        self.literal_memory_end += len;

        // Save the offset in the literal memory for this string
        self.literal_memory_offset.insert(entry, offset);

        (offset, len)
    }

    /// Adds a new string literal into the memory for an identifier
    pub(crate) fn add_string_literal(&mut self, name: &str) -> (u32, u32) {
        // If this identifier has already been saved in the literal memory,
        // just return the offset and length.
        let entry = LiteralMemoryEntry::Ascii(name.to_string());
        if let Some(offset) = self.literal_memory_offset.get(&entry) {
            return (*offset, name.len() as u32);
        }

        let memory = self.module.memories.iter().next().expect("no memory found");
        let offset = self.literal_memory_end;
        let len = name.len() as u32;
        self.module.data.add(
            DataKind::Active(ActiveData {
                memory: memory.id(),
                location: walrus::ActiveDataLocation::Absolute(offset),
            }),
            name.as_bytes().to_vec(),
        );
        self.literal_memory_end += name.len() as u32;

        // Save the offset in the literal memory for this identifier
        self.literal_memory_offset.insert(entry, offset);

        (offset, len)
    }

    /// Adds a new literal into the memory, and returns the offset and length.
    pub(crate) fn add_literal(&mut self, value: &clarity::vm::Value) -> (u32, u32) {
        let data = match value {
            clarity::vm::Value::Int(i) => {
                let mut data = (((*i as u128) & 0xFFFFFFFFFFFFFFFF) as i64)
                    .to_le_bytes()
                    .to_vec();
                data.extend_from_slice(&(((*i as u128) >> 64) as i64).to_le_bytes());
                data
            }
            clarity::vm::Value::UInt(u) => {
                let mut data = ((*u & 0xFFFFFFFFFFFFFFFF) as i64).to_le_bytes().to_vec();
                data.extend_from_slice(&((*u >> 64) as i64).to_le_bytes());
                data
            }
            clarity::vm::Value::Principal(p) => match p {
                PrincipalData::Standard(standard) => {
                    let mut data = vec![standard.0];
                    data.extend_from_slice(&standard.1);
                    // Append a 0 for the length of the contract name
                    data.push(0);
                    data
                }
                PrincipalData::Contract(contract) => {
                    let mut data = vec![contract.issuer.0];
                    data.extend_from_slice(&contract.issuer.1);
                    let contract_length = contract.name.len();
                    data.push(contract_length);
                    data.extend_from_slice(contract.name.as_bytes());
                    data
                }
            },
            clarity::vm::Value::Sequence(SequenceData::Buffer(buff_data)) => buff_data.data.clone(),
            clarity::vm::Value::Sequence(SequenceData::String(string_data)) => {
                return self.add_clarity_string_literal(string_data);
            }
            _ => unimplemented!("Unsupported literal: {}", value),
        };
        let memory = self.module.memories.iter().next().expect("no memory found");
        let offset = self.literal_memory_end;
        let len = data.len() as u32;
        self.module.data.add(
            DataKind::Active(ActiveData {
                memory: memory.id(),
                location: walrus::ActiveDataLocation::Absolute(offset),
            }),
            data.clone(),
        );
        self.literal_memory_end += data.len() as u32;

        (offset, len)
    }

    pub(crate) fn block_from_expr(
        &mut self,
        builder: &mut InstrSeqBuilder,
        expr: &SymbolicExpression,
    ) -> Result<InstrSeqId, GeneratorError> {
        let return_type = clar2wasm_ty(
            self.get_expr_type(expr)
                .expect("Expression results must be typed"),
        );

        let mut block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            &return_type,
        ));
        self.traverse_expr(&mut block, expr)?;

        Ok(block.id())
    }

    /// Push a new local onto the call stack, adjusting the stack pointer and
    /// tracking the current function's frame size accordingly.
    /// - `include_repr` indicates if space should be reserved for the
    ///   representation of the value (e.g. the offset, length for an in-memory
    ///   type)
    /// - `include_value` indicates if space should be reserved for the value
    ///
    /// Returns a local which is a pointer to the beginning of the allocated
    /// stack space and the size of the allocated space.
    pub(crate) fn create_call_stack_local(
        &mut self,
        builder: &mut InstrSeqBuilder,
        ty: &TypeSignature,
        include_repr: bool,
        include_value: bool,
    ) -> (LocalId, i32) {
        let size = match (include_value, include_repr) {
            (true, true) => get_type_in_memory_size(ty, include_repr) + get_type_size(ty),
            (true, false) => get_type_in_memory_size(ty, include_repr),
            (false, true) => get_type_size(ty),
            (false, false) => unreachable!("must include either repr or value"),
        };

        // Save the offset (current stack pointer) into a local
        let offset = self.module.locals.add(ValType::I32);
        builder
            // []
            .global_get(self.stack_pointer)
            // [ stack_ptr ]
            .local_tee(offset);
        // [ stack_ptr ]

        // TODO: The frame stack size can be computed at compile time, so we
        //       should be able to increment the stack pointer once in the function
        //       prelude with a constant instead of incrementing it for each local.
        // (global.set $stack-pointer (i32.add (global.get $stack-pointer) (i32.const <size>))
        builder
            // [ stack_ptr ]
            .i32_const(size)
            // [ stack_ptr, size ]
            .binop(BinaryOp::I32Add)
            // [ new_stack_ptr ]
            .global_set(self.stack_pointer);
        // [  ]
        self.frame_size += size;

        (offset, size)
    }

    /// Write the value that is on the top of the data stack, which has type
    /// `ty`, to the memory, at offset stored in local variable,
    /// `offset_local`, plus constant offset `offset`. Returns the number of
    /// bytes written.
    pub(crate) fn write_to_memory(
        &mut self,
        builder: &mut InstrSeqBuilder,
        offset_local: LocalId,
        offset: u32,
        ty: &TypeSignature,
    ) -> u32 {
        let memory = self.module.memories.iter().next().expect("no memory found");
        match ty {
            TypeSignature::IntType | TypeSignature::UIntType => {
                // Data stack: TOP | High | Low | ...
                // Save the high/low to locals.
                let high = self.module.locals.add(ValType::I64);
                let low = self.module.locals.add(ValType::I64);
                builder.local_set(high).local_set(low);

                // Store the high/low to memory.
                builder.local_get(offset_local).local_get(low).store(
                    memory.id(),
                    StoreKind::I64 { atomic: false },
                    MemArg { align: 8, offset },
                );
                builder.local_get(offset_local).local_get(high).store(
                    memory.id(),
                    StoreKind::I64 { atomic: false },
                    MemArg {
                        align: 8,
                        offset: offset + 8,
                    },
                );
                16
            }
            TypeSignature::PrincipalType
            | TypeSignature::SequenceType(SequenceSubtype::BufferType(_))
            | TypeSignature::SequenceType(SequenceSubtype::ListType(_))
            | TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                // Data stack: TOP | Length | Offset | ...
                // Save the offset/length to locals.
                let seq_offset = self.module.locals.add(ValType::I32);
                let seq_length = self.module.locals.add(ValType::I32);
                builder.local_set(seq_length).local_set(seq_offset);

                // Store the offset/length to memory.
                builder.local_get(offset_local).local_get(seq_offset).store(
                    memory.id(),
                    StoreKind::I32 { atomic: false },
                    MemArg { align: 4, offset },
                );
                builder.local_get(offset_local).local_get(seq_length).store(
                    memory.id(),
                    StoreKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: offset + 4,
                    },
                );
                8
            }
            TypeSignature::BoolType => {
                // Data stack: TOP | Value | ...
                // Save the value to a local.
                let bool_val = self.module.locals.add(ValType::I32);
                builder.local_set(bool_val);

                // Store the value to memory.
                builder.local_get(offset_local).local_get(bool_val).store(
                    memory.id(),
                    StoreKind::I32 { atomic: false },
                    MemArg { align: 4, offset },
                );
                4
            }
            TypeSignature::NoType => {
                // Data stack: TOP | (Place holder i32)
                // We just have to drop the placeholder and write a i32
                builder.drop().local_get(offset_local).i32_const(0).store(
                    memory.id(),
                    StoreKind::I32 { atomic: false },
                    MemArg { align: 4, offset },
                );
                4
            }
            TypeSignature::OptionalType(some_ty) => {
                let memory_id = memory.id();
                // Data stack: TOP | inner value | (some|none) variant
                // recursively store the inner value

                let bytes_written =
                    self.write_to_memory(builder, offset_local, offset + 4, some_ty);

                // Save the variant to a local and store it to memory
                let variant_val = self.module.locals.add(ValType::I32);
                builder
                    .local_set(variant_val)
                    .local_get(offset_local)
                    .local_get(variant_val)
                    .store(
                        memory_id,
                        StoreKind::I32 { atomic: false },
                        MemArg { align: 4, offset },
                    );

                // recursively store the inner value
                4 + bytes_written
            }
            TypeSignature::ResponseType(ok_err_ty) => {
                // Data stack: TOP | err_value | ok_value | (ok|err) variant
                let memory_id = memory.id();
                let mut bytes_written = 0;

                // write err value at offset + size of variant (4) + size of ok_value
                bytes_written += self.write_to_memory(
                    builder,
                    offset_local,
                    offset + 4 + get_type_size(&ok_err_ty.0) as u32,
                    &ok_err_ty.1,
                );

                // write ok value at offset + size of variant (4)
                bytes_written +=
                    self.write_to_memory(builder, offset_local, offset + 4, &ok_err_ty.0);

                let variant_val = self.module.locals.add(ValType::I32);
                builder
                    .local_set(variant_val)
                    .local_get(offset_local)
                    .local_get(variant_val)
                    .store(
                        memory_id,
                        StoreKind::I32 { atomic: false },
                        MemArg { align: 4, offset },
                    );

                bytes_written + 4
            }
            TypeSignature::TupleType(tuple_ty) => {
                // Data stack: TOP | last_value | value_before_last | ... | first_value
                // we will write the values from last to first by setting the correct offset at which it's supposed to be written
                let mut bytes_written = 0;
                let types: Vec<_> = tuple_ty.get_type_map().values().collect();
                let offsets_delta: Vec<_> = std::iter::once(0u32)
                    .chain(
                        types
                            .iter()
                            .map(|t| get_type_size(t) as u32)
                            .scan(0, |acc, i| {
                                *acc += i;
                                Some(*acc)
                            }),
                    )
                    .collect();
                for (elem_ty, offset_delta) in types.into_iter().zip(offsets_delta).rev() {
                    bytes_written +=
                        self.write_to_memory(builder, offset_local, offset + offset_delta, elem_ty);
                }
                bytes_written
            }
            _ => unimplemented!("Type not yet supported for writing to memory: {ty}"),
        }
    }

    /// Read a value from memory at offset stored in local variable `offset`,
    /// with type `ty`, and push it onto the top of the data stack.
    pub(crate) fn read_from_memory(
        &mut self,
        builder: &mut InstrSeqBuilder,
        offset: LocalId,
        literal_offset: u32,
        ty: &TypeSignature,
    ) -> i32 {
        let memory = self.module.memories.iter().next().expect("no memory found");
        let size = match ty {
            TypeSignature::IntType | TypeSignature::UIntType => {
                // Memory: Offset -> | Low | High |
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I64 { atomic: false },
                    MemArg {
                        align: 8,
                        offset: literal_offset,
                    },
                );
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I64 { atomic: false },
                    MemArg {
                        align: 8,
                        offset: literal_offset + 8,
                    },
                );
                16
            }
            TypeSignature::OptionalType(inner) => {
                // Memory: Offset -> | Indicator | Value |
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: literal_offset,
                    },
                );
                4 + self.read_from_memory(builder, offset, literal_offset + 4, inner)
            }
            TypeSignature::ResponseType(inner) => {
                // Memory: Offset -> | Indicator | Ok Value | Err Value |
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: literal_offset,
                    },
                );
                let mut offset_adjust = 4;
                offset_adjust += self.read_from_memory(
                    builder,
                    offset,
                    literal_offset + offset_adjust,
                    &inner.0,
                ) as u32;
                offset_adjust += self.read_from_memory(
                    builder,
                    offset,
                    literal_offset + offset_adjust,
                    &inner.1,
                ) as u32;
                offset_adjust as i32
            }
            // Principals and sequence types are stored in-memory and
            // represented by an offset and length.
            TypeSignature::PrincipalType | TypeSignature::SequenceType(_) => {
                // Memory: Offset -> | ValueOffset | ValueLength |
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: literal_offset,
                    },
                );
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: literal_offset + 4,
                    },
                );
                8
            }
            TypeSignature::TupleType(tuple) => {
                // Memory: Offset -> | Value1 | Value2 | ... |
                let mut offset_adjust = 0;
                for ty in tuple.get_type_map().values() {
                    offset_adjust +=
                        self.read_from_memory(builder, offset, literal_offset + offset_adjust, ty)
                            as u32;
                }
                offset_adjust as i32
            }
            // Unknown types just get a placeholder i32 value.
            TypeSignature::NoType => {
                builder.i32_const(0);
                4
            }
            TypeSignature::BoolType => {
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: literal_offset,
                    },
                );
                4
            }
            _ => unimplemented!("Type not yet supported for reading from memory: {ty}"),
        };
        size
    }

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
        builder.local_get(high).call(
            self.module
                .funcs
                .by_name("stdlib.store-i64-be")
                .expect("store-i64-be not found"),
        );

        // Adjust the write pointer
        builder
            .local_get(write_ptr)
            .i32_const(8)
            .binop(BinaryOp::I32Add)
            .local_tee(write_ptr);
        written += 8;

        // Adjust the offset by 8, then serialize the low to memory.
        builder.local_get(low).call(
            self.module
                .funcs
                .by_name("stdlib.store-i64-be")
                .expect("store-i64-be not found"),
        );
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

        // Now serialize the ok value to memory
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
            size_non_zero.call(
                self.module
                    .funcs
                    .by_name("stdlib.store-i32-be")
                    .expect("store-i32-be not found"),
            );

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

                self.read_from_memory(&mut loop_, read_ptr, 0, element_ty);

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
        builder.local_get(length).call(
            self.module
                .funcs
                .by_name("stdlib.store-i32-be")
                .expect("store-i32-be not found"),
        );

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
        builder.local_get(length).call(
            self.module
                .funcs
                .by_name("stdlib.store-i32-be")
                .expect("store-i32-be not found"),
        );

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
            .call(
                self.module
                    .funcs
                    .by_name("stdlib.convert-scalars-to-utf8")
                    .expect("convert-scalars-to-utf8 not found"),
            )
            .local_tee(utf8_length);

        // Serialize the length to memory (big endian)
        builder
            .local_get(offset_local)
            .i32_const(offset as i32 + 1)
            .binop(BinaryOp::I32Add)
            .local_get(utf8_length)
            .call(
                self.module
                    .funcs
                    .by_name("stdlib.store-i32-be")
                    .expect("store-i32-be not found"),
            );

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
            .call(
                self.module
                    .funcs
                    .by_name("stdlib.store-i32-be")
                    .expect("store-i32-be not found"),
            );

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
                    locals
                        .pop()
                        .ok_or(GeneratorError::InternalError("invalid tuple value".into()))?,
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
        builder.local_get(offset_local).binop(BinaryOp::I32Sub);

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
                    let result_offset = self.module.locals.add(ValType::I32);
                    then.global_get(self.stack_pointer).local_tee(result_offset);
                    then.i32_const(STANDARD_PRINCIPAL_BYTES as i32)
                        .binop(BinaryOp::I32Add)
                        .global_set(self.stack_pointer);

                    // Copy the principal to the destination
                    then.local_get(result_offset)
                        .local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .i32_const(PRINCIPAL_BYTES as i32)
                        .memory_copy(memory, memory);

                    // Write the contract name length (0)
                    then.local_get(result_offset).i32_const(0).store(
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
                    then.local_get(result_offset)
                        .i32_const(PRINCIPAL_BYTES as i32);

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
        self.deserialize_from_memory(&mut some_block, offset_local, end_local, value_ty)?;

        // Check if the deserialization failed:
        // - Store the value in locals
        // - Check the inidicator now on top of the stack
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
    fn deserialize_response(
        &mut self,
        builder: &mut InstrSeqBuilder,
        memory: MemoryId,
        offset_local: LocalId,
        end_local: LocalId,
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
        self.deserialize_from_memory(&mut ok_block, offset_local, end_local, ok_ty)?;

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
        self.deserialize_from_memory(&mut err_block, offset_local, end_local, err_ty)?;

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
            .call(
                self.module
                    .funcs
                    .by_name("stdlib.load-i32-be")
                    .expect("load-i32-be not found"),
            )
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
        let result_offset = self.module.locals.add(ValType::I32);
        let element_size = get_type_size(element_ty);
        block
            .global_get(self.stack_pointer)
            .local_tee(result)
            .local_tee(result_offset);
        block
            .local_get(length)
            .i32_const(element_size)
            .binop(BinaryOp::I32Mul)
            .binop(BinaryOp::I32Add)
            .global_set(self.stack_pointer);

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
        self.deserialize_from_memory(&mut loop_block, offset_local, end_local, element_ty)?;

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
        let bytes_written = self.write_to_memory(&mut loop_block, result_offset, 0, element_ty);

        // Increment the result offset by the number of bytes written
        loop_block
            .local_get(result_offset)
            .i32_const(bytes_written as i32)
            .binop(BinaryOp::I32Add)
            .local_set(result_offset);

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
        tuple_ty: &TupleTypeSignature,
    ) -> Result<(), GeneratorError> {
        let ty = TypeSignature::TupleType(tuple_ty.clone());

        // Create a block for the body of this operation, so that we can
        // early exit as needed.
        let mut wasm_val_ty = vec![ValType::I32];
        wasm_val_ty.extend(clar2wasm_ty(&ty));
        let block_ty = InstrSeqType::new(&mut self.module.types, &[], &wasm_val_ty);
        let mut block = builder.dangling_instr_seq(block_ty);
        let block_id = block.id();

        // Verify that reading 5 bytes (prefix + number of keys) from the
        // offset is within the buffer.
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
                    then.i32_const(0);
                    add_placeholder_for_clarity_type(then, &ty);
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
            .i32_const(TypePrefix::Tuple as i32)
            .binop(BinaryOp::I32Ne)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0);
                    add_placeholder_for_clarity_type(then, &ty);
                    then.br(block_id);
                },
                |_| {},
            );

        // Read the number of keys
        block
            .local_get(offset_local)
            .i32_const(1)
            .binop(BinaryOp::I32Add)
            .call(
                self.module
                    .funcs
                    .by_name("stdlib.load-i32-be")
                    .expect("load-i32-be not found"),
            );

        // Verify that the number of keys matches the specified type
        block
            .i32_const(tuple_ty.get_type_map().len() as i32)
            .binop(BinaryOp::I32Ne)
            .if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0);
                    add_placeholder_for_clarity_type(then, &ty);
                    then.br(block_id);
                },
                |_| {},
            );

        // Update the offset to point to the key
        block
            .local_get(offset_local)
            .i32_const(5)
            .binop(BinaryOp::I32Add)
            .local_set(offset_local);

        // For each key in the type, verify that the key matches the type,
        // and deserialize the value.
        for (key, value_ty) in tuple_ty.get_type_map() {
            // The key is a 1-byte length followed by the string bytes.
            // First, verify that the key is within the buffer.
            block
                .local_get(offset_local)
                .i32_const(key.len() as i32 + 1)
                .binop(BinaryOp::I32Add)
                .local_get(end_local)
                .binop(BinaryOp::I32GtU)
                .if_else(
                    None,
                    |then| {
                        // Return none
                        then.i32_const(0);
                        add_placeholder_for_clarity_type(then, &ty);
                        then.br(block_id);
                    },
                    |_| {},
                );

            // Then, grab the length of the key.
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

            // Compare the key length to the expected length
            block
                .i32_const(key.len() as i32)
                .binop(BinaryOp::I32Ne)
                .if_else(
                    None,
                    |then| {
                        // Return none
                        then.i32_const(0);
                        add_placeholder_for_clarity_type(then, &ty);
                        then.br(block_id);
                    },
                    |_| {},
                );

            // Compare the key to the expected key
            let key_bytes = key.as_bytes();
            for (i, byte) in key_bytes.iter().enumerate() {
                block
                    .local_get(offset_local)
                    .load(
                        memory,
                        LoadKind::I32_8 {
                            kind: ExtendedLoad::ZeroExtend,
                        },
                        MemArg {
                            align: 1,
                            offset: i as u32 + 1,
                        },
                    )
                    .i32_const(*byte as i32)
                    .binop(BinaryOp::I32Ne)
                    .if_else(
                        None,
                        |then| {
                            // Return none
                            then.i32_const(0);
                            add_placeholder_for_clarity_type(then, &ty);
                            then.br(block_id);
                        },
                        |_| {},
                    );
            }

            // Increment the offset by the key length and its size
            block
                .local_get(offset_local)
                .i32_const(key.len() as i32 + 1)
                .binop(BinaryOp::I32Add)
                .local_set(offset_local);

            // Deserialize the value. Note, this will update the offset to
            // point to the next key.
            self.deserialize_from_memory(&mut block, offset_local, end_local, value_ty)?;

            // Check if the deserialization failed:
            // - Store the value in locals
            // - Check the inidicator now on top of the stack
            let inner_locals = self.save_to_locals(&mut block, value_ty, true);

            block.unop(UnaryOp::I32Eqz).if_else(
                None,
                |then| {
                    // Return none
                    then.i32_const(0);
                    add_placeholder_for_clarity_type(then, &ty);
                    then.br(block_id);
                },
                |_| {},
            );

            // Deserializing the element was successful, so push the value back
            // onto the stack.
            for local in inner_locals {
                block.local_get(local);
            }
        }

        // If we've reached here, then the tuple is valid, so return it.
        // But first we need to push the `some` indicator onto the stack.

        // Save the tuple (on the stack) to locals
        let tuple_locals = self.save_to_locals(&mut block, &ty, true);

        // Push the `some` indicator onto the stack
        block.i32_const(1);

        // Push the tuple back onto the stack
        for local in tuple_locals {
            block.local_get(local);
        }

        // Add our main block to the builder.
        builder.instr(walrus::ir::Block { seq: block_id });

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
            .binop(BinaryOp::I32GeU)
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
                        .call(
                            self.module
                                .funcs
                                .by_name("stdlib.load-i32-be")
                                .expect("load-i32-be not found"),
                        )
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
            .binop(BinaryOp::I32GeU)
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
                        .call(
                            self.module
                                .funcs
                                .by_name("stdlib.load-i32-be")
                                .expect("load-i32-be not found"),
                        )
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
            .binop(BinaryOp::I32GeU)
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
            .i32_const(TypePrefix::StringUTF8 as i32)
            .binop(BinaryOp::I32Eq)
            .if_else(
                block_ty,
                |then| {
                    // Read the string length
                    let string_length = self.module.locals.add(ValType::I32);
                    then.local_get(offset_local)
                        .i32_const(1)
                        .binop(BinaryOp::I32Add)
                        .call(
                            self.module
                                .funcs
                                .by_name("stdlib.load-i32-be")
                                .expect("load-i32-be not found"),
                        )
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

                    // Reserve space in the call stack to hold the deserialized
                    // string.
                    let result_offset = self.module.locals.add(ValType::I32);
                    then.global_get(self.stack_pointer).local_tee(result_offset);
                    then.i32_const((type_length * 4) as i32)
                        .binop(BinaryOp::I32Add)
                        .global_set(self.stack_pointer);

                    // Push the offset of the result onto the stack
                    then.local_get(result_offset);

                    // Call utf8 to scalar conversion function. It will leave
                    // return the length of the string.
                    then.local_get(offset_local)
                        .i32_const(5)
                        .binop(BinaryOp::I32Add)
                        .local_get(string_length)
                        .local_get(result_offset)
                        .call(
                            self.module
                                .funcs
                                .by_name("stdlib.convert-utf8-to-scalars")
                                .expect("stdlib.convert-utf8-to-scalars not found"),
                        );

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
        ty: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let memory = self.get_memory();

        use clarity::vm::types::signatures::TypeSignature::*;
        match ty {
            IntType | UIntType => {
                self.deserialize_integer(builder, memory, offset_local, end_local, ty == &IntType)
            }
            PrincipalType | CallableType(_) | TraitReferenceType(_) => {
                self.deserialize_principal(builder, memory, offset_local, end_local)
            }
            ResponseType(types) => self.deserialize_response(
                builder,
                memory,
                offset_local,
                end_local,
                &types.0,
                &types.1,
            ),
            BoolType => self.deserialize_bool(builder, memory, offset_local, end_local),
            OptionalType(value_ty) => {
                self.deserialize_optional(builder, memory, offset_local, end_local, value_ty)
            }
            SequenceType(SequenceSubtype::ListType(list_ty)) => {
                self.deserialize_list(builder, memory, offset_local, end_local, list_ty)
            }
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
            SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(type_length))) => self
                .deserialize_string_utf8(
                    builder,
                    memory,
                    offset_local,
                    end_local,
                    type_length.into(),
                ),
            TupleType(tuple_ty) => {
                self.deserialize_tuple(builder, memory, offset_local, end_local, tuple_ty)
            }
            NoType => unreachable!("NoType should not be deserialized"),
            ListUnionType(_) => unreachable!("ListUnionType should not be deserialized"),
        }
    }

    pub(crate) fn traverse_statement_list(
        &mut self,
        builder: &mut InstrSeqBuilder,
        statements: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        assert!(
            !statements.is_empty(),
            "statement list must have at least one statement"
        );
        // Traverse all but the last statement and drop any unused values.
        for stmt in &statements[..statements.len() - 1] {
            self.traverse_expr(builder, stmt)?;
            // If stmt has a type, and is not the last statement, its value
            // needs to be discarded.
            if let Some(ty) = self.get_expr_type(stmt) {
                drop_value(builder.borrow_mut(), ty);
            }
        }

        // Traverse the last statement in the block, whose result is the result
        // of the `begin` expression.
        self.traverse_expr(builder, statements.last().unwrap())
    }

    /// If `name` is a reserved variable, push its value onto the data stack.
    pub fn lookup_reserved_variable(
        &mut self,
        builder: &mut InstrSeqBuilder,
        name: &str,
        expr: &SymbolicExpression,
    ) -> bool {
        if let Some(variable) = NativeVariables::lookup_by_name_at_version(
            name,
            &self.contract_analysis.clarity_version,
        ) {
            match variable {
                NativeVariables::TxSender => {
                    // Create a new local to hold the result on the call stack
                    let (offset, size);
                    (offset, size) = self.create_call_stack_local(
                        builder,
                        &TypeSignature::PrincipalType,
                        false,
                        true,
                    );

                    // Push the offset and size to the data stack
                    builder.local_get(offset).i32_const(size);

                    // Call the host interface function, `tx_sender`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.tx_sender")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::ContractCaller => {
                    // Create a new local to hold the result on the call stack
                    let (offset, size);
                    (offset, size) = self.create_call_stack_local(
                        builder,
                        &TypeSignature::PrincipalType,
                        false,
                        true,
                    );

                    // Push the offset and size to the data stack
                    builder.local_get(offset).i32_const(size);

                    // Call the host interface function, `contract_caller`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.contract_caller")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::TxSponsor => {
                    // Create a new local to hold the result on the call stack
                    let (offset, size);
                    (offset, size) = self.create_call_stack_local(
                        builder,
                        &TypeSignature::PrincipalType,
                        false,
                        true,
                    );

                    // Push the offset and size to the data stack
                    builder.local_get(offset).i32_const(size);

                    // Call the host interface function, `tx_sponsor`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.tx_sponsor")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::BlockHeight => {
                    // Call the host interface function, `block_height`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.block_height")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::BurnBlockHeight => {
                    // Call the host interface function, `burn_block_height`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.burn_block_height")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::NativeNone => {
                    let ty = self.get_expr_type(expr).expect("'none' must be typed");
                    add_placeholder_for_clarity_type(builder, ty);
                    true
                }
                NativeVariables::NativeTrue => {
                    builder.i32_const(1);
                    true
                }
                NativeVariables::NativeFalse => {
                    builder.i32_const(0);
                    true
                }
                NativeVariables::TotalLiquidMicroSTX => {
                    // Call the host interface function, `stx_liquid_supply`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.stx_liquid_supply")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::Regtest => {
                    // Call the host interface function, `is_in_regtest`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.is_in_regtest")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::Mainnet => {
                    // Call the host interface function, `is_in_mainnet`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.is_in_mainnet")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::ChainId => {
                    // Call the host interface function, `chain_id`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stdlib.chain_id")
                            .expect("function not found"),
                    );
                    true
                }
            }
        } else {
            false
        }
    }

    /// If `name` is a constant, push its value onto the data stack.
    pub fn lookup_constant_variable(
        &mut self,
        builder: &mut InstrSeqBuilder,
        name: &str,
        expr: &SymbolicExpression,
    ) -> bool {
        if let Some(offset) = self.constants.get(name) {
            // Load the offset into a local variable
            let offset_local = self.module.locals.add(ValType::I32);
            builder.i32_const(*offset as i32).local_set(offset_local);

            let ty = self
                .get_expr_type(expr)
                .expect("constant must be typed")
                .clone();

            // If `ty` is a value that stays in memory, we can just push the
            // offset and length to the stack.
            if is_in_memory_type(&ty) {
                builder
                    .local_get(offset_local)
                    .i32_const(get_type_in_memory_size(&ty, false));
                true
            } else {
                // Otherwise, we need to load the value from memory.
                self.read_from_memory(builder, offset_local, 0, &ty);
                true
            }
        } else {
            false
        }
    }

    /// Save the expression on the top of the stack, with Clarity type `ty`, to
    /// local variables. If `fix_ordering` is true, then the vector is reversed
    /// so that the types are in logical order. Without this, they will be in
    /// reverse order, due to the order we pop values from the stack. Return
    /// the list of local variables.
    pub fn save_to_locals(
        &mut self,
        builder: &mut walrus::InstrSeqBuilder,
        ty: &TypeSignature,
        fix_ordering: bool,
    ) -> Vec<LocalId> {
        let wasm_types = clar2wasm_ty(ty);
        let mut locals = Vec::with_capacity(wasm_types.len());
        // Iterate in reverse order, since we are popping items off of the top
        // in reverse order.
        for wasm_ty in wasm_types.iter().rev() {
            let local = self.module.locals.add(*wasm_ty);
            locals.push(local);
            builder.local_set(local);
        }

        if fix_ordering {
            // Reverse the locals to put them back in the correct order.
            locals.reverse();
        }
        locals
    }
}

/// Convert a Clarity type signature to a wasm type signature.
pub(crate) fn clar2wasm_ty(ty: &TypeSignature) -> Vec<ValType> {
    match ty {
        TypeSignature::NoType => vec![ValType::I32], // TODO: can this just be empty?
        TypeSignature::IntType => vec![ValType::I64, ValType::I64],
        TypeSignature::UIntType => vec![ValType::I64, ValType::I64],
        TypeSignature::ResponseType(inner_types) => {
            let mut types = vec![ValType::I32];
            types.extend(clar2wasm_ty(&inner_types.0));
            types.extend(clar2wasm_ty(&inner_types.1));
            types
        }
        TypeSignature::SequenceType(_) => vec![
            ValType::I32, // offset
            ValType::I32, // length
        ],
        TypeSignature::BoolType => vec![ValType::I32],
        TypeSignature::PrincipalType | TypeSignature::CallableType(_) => vec![
            ValType::I32, // offset
            ValType::I32, // length
        ],
        TypeSignature::OptionalType(inner_ty) => {
            let mut types = vec![ValType::I32];
            types.extend(clar2wasm_ty(inner_ty));
            types
        }
        TypeSignature::TupleType(inner_types) => {
            let mut types = vec![];
            for inner_type in inner_types.get_type_map().values() {
                types.extend(clar2wasm_ty(inner_type));
            }
            types
        }
        _ => unimplemented!("{:?}", ty),
    }
}

/// Drop a value of type `ty` from the data stack.
pub(crate) fn drop_value(builder: &mut InstrSeqBuilder, ty: &TypeSignature) {
    let wasm_types = clar2wasm_ty(ty);
    (0..wasm_types.len()).for_each(|_| {
        builder.drop();
    });
}

impl WasmGenerator {
    pub fn func_by_name(&self, name: &str) -> FunctionId {
        self.module
            .funcs
            .by_name(name)
            .unwrap_or_else(|| panic!("function not found: {name}"))
    }

    fn visit_literal_value(
        &mut self,
        builder: &mut InstrSeqBuilder,
        _expr: &SymbolicExpression,
        value: &clarity::vm::Value,
    ) -> Result<(), GeneratorError> {
        match value {
            clarity::vm::Value::Int(i) => {
                builder.i64_const((i & 0xFFFFFFFFFFFFFFFF) as i64);
                builder.i64_const(((i >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                Ok(())
            }
            clarity::vm::Value::UInt(u) => {
                builder.i64_const((u & 0xFFFFFFFFFFFFFFFF) as i64);
                builder.i64_const(((u >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                Ok(())
            }
            clarity::vm::Value::Sequence(SequenceData::String(s)) => {
                let (offset, len) = self.add_clarity_string_literal(s);
                builder.i32_const(offset as i32);
                builder.i32_const(len as i32);
                Ok(())
            }
            clarity::vm::Value::Principal(_)
            | clarity::vm::Value::Sequence(SequenceData::Buffer(_)) => {
                let (offset, len) = self.add_literal(value);
                builder.i32_const(offset as i32);
                builder.i32_const(len as i32);
                Ok(())
            }
            _ => Err(GeneratorError::NotImplemented),
        }
    }

    fn visit_atom(
        &mut self,
        builder: &mut InstrSeqBuilder,
        expr: &SymbolicExpression,
        atom: &ClarityName,
    ) -> Result<(), GeneratorError> {
        // Handle builtin variables
        if self.lookup_reserved_variable(builder, atom.as_str(), expr) {
            return Ok(());
        }

        if self.lookup_constant_variable(builder, atom.as_str(), expr) {
            return Ok(());
        }

        // Handle parameters and local bindings
        let values = self
            .bindings
            .get(atom.as_str())
            .ok_or(GeneratorError::InternalError(format!(
                "unable to find local for {}",
                atom.as_str()
            )))?;

        for value in values {
            builder.local_get(*value);
        }

        Ok(())
    }

    fn traverse_call_user_defined(
        &mut self,
        builder: &mut InstrSeqBuilder,
        expr: &SymbolicExpression,
        name: &ClarityName,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        self.traverse_args(builder, args)?;

        let return_ty = self
            .get_expr_type(expr)
            .expect("function call expression must be typed")
            .clone();
        self.visit_call_user_defined(builder, &return_ty, name)
    }

    /// Visit a function call to a user-defined function. Arguments must have
    /// already been traversed and pushed to the stack.
    pub fn visit_call_user_defined(
        &mut self,
        builder: &mut InstrSeqBuilder,
        return_ty: &TypeSignature,
        name: &ClarityName,
    ) -> Result<(), GeneratorError> {
        if self
            .contract_analysis
            .get_public_function_type(name.as_str())
            .is_some()
        {
            self.local_call_public(builder, return_ty, name)?;
        } else if self
            .contract_analysis
            .get_read_only_function_type(name.as_str())
            .is_some()
        {
            self.local_call_read_only(builder, name)?;
        } else if self
            .contract_analysis
            .get_private_function(name.as_str())
            .is_some()
        {
            self.local_call(builder, name)?;
        } else {
            return Err(GeneratorError::TypeError(format!(
                "function not found: {name}",
                name = name.as_str()
            )));
        }

        // If an in-memory value is returned from a function, we need to copy
        // it to our frame, from the callee's frame.
        if is_in_memory_type(return_ty) {
            // The result may be in the callee's call frame, can be overwritten
            // after returning, so we need to copy it to our frame.
            let result_offset = self.module.locals.add(ValType::I32);
            let result_length = self.module.locals.add(ValType::I32);
            builder.local_set(result_length).local_set(result_offset);

            // Reserve space to store the returned value.
            let offset = self.module.locals.add(ValType::I32);
            builder.global_get(self.stack_pointer).local_tee(offset);
            builder
                .local_get(result_length)
                .binop(BinaryOp::I32Add)
                .global_set(self.stack_pointer);

            // Copy the result to our frame.
            builder
                .local_get(offset)
                .local_get(result_offset)
                .local_get(result_length)
                .memory_copy(self.get_memory(), self.get_memory());

            // Push the copied offset and length to the stack
            builder.local_get(offset).local_get(result_length);
        }

        Ok(())
    }

    /// Call a function defined in the current contract.
    fn local_call(
        &mut self,
        builder: &mut InstrSeqBuilder,
        name: &ClarityName,
    ) -> Result<(), GeneratorError> {
        builder.call(
            self.module
                .funcs
                .by_name(name.as_str())
                .ok_or(GeneratorError::TypeError(format!(
                    "function not found: {name}"
                )))?,
        );

        Ok(())
    }

    /// Call a public function defined in the current contract. This requires
    /// going through the host interface to handle roll backs.
    fn local_call_public(
        &mut self,
        builder: &mut InstrSeqBuilder,
        return_ty: &TypeSignature,
        name: &ClarityName,
    ) -> Result<(), GeneratorError> {
        // Call the host interface function, `begin_public_call`
        builder.call(
            self.module
                .funcs
                .by_name("stdlib.begin_public_call")
                .expect("function not found"),
        );

        self.local_call(builder, name)?;

        // Save the result to a local
        let result_locals = self.save_to_locals(builder, return_ty, true);

        // If the result is an `ok`, then we can commit the call, and if it
        // is an `err`, then we roll it back. `result_locals[0]` is the
        // response indicator (all public functions return a response).
        builder.local_get(result_locals[0]).if_else(
            None,
            |then| {
                // Call the host interface function, `commit_call`
                then.call(
                    self.module
                        .funcs
                        .by_name("stdlib.commit_call")
                        .expect("function not found"),
                );
            },
            |else_| {
                // Call the host interface function, `roll_back_call`
                else_.call(
                    self.module
                        .funcs
                        .by_name("stdlib.roll_back_call")
                        .expect("function not found"),
                );
            },
        );

        // Restore the result to the top of the stack.
        for local in &result_locals {
            builder.local_get(*local);
        }

        Ok(())
    }

    /// Call a read-only function defined in the current contract.
    fn local_call_read_only(
        &mut self,
        builder: &mut InstrSeqBuilder,
        name: &ClarityName,
    ) -> Result<(), GeneratorError> {
        // Call the host interface function, `begin_readonly_call`
        builder.call(
            self.module
                .funcs
                .by_name("stdlib.begin_read_only_call")
                .expect("function not found"),
        );

        self.local_call(builder, name)?;

        // Call the host interface function, `roll_back_call`
        builder.call(
            self.module
                .funcs
                .by_name("stdlib.roll_back_call")
                .expect("function not found"),
        );

        Ok(())
    }

    pub fn traverse_args(
        &mut self,
        builder: &mut InstrSeqBuilder,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        for arg in args.iter() {
            self.traverse_expr(builder, arg)?;
        }
        Ok(())
    }
}
