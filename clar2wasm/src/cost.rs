//! Functionality to track the costs of running Clarity.
//!
//! The cost computations in this module are meant to be a full match with the interpreter
//! implementation of the Clarity runtime.

use std::ptr;

use walrus::ir::{BinaryOp, Binop, Instr, UnaryOp, Unop};
use walrus::{FunctionId, GlobalId, InstrSeqBuilder, LocalId, ValType};

use crate::error_mapping::ErrorMap;
use crate::wasm_generator::WasmGenerator;

/// Generators of cost tracking code.
pub trait CostTrackingGenerator {
    /// The cost tracking context.
    ///
    /// Shouldn't be called externally.
    fn cost_context(&mut self) -> &mut CostTrackingContext;

    /// Produce `n` new locals to be used for cost tracking, and push them into the context.
    ///
    /// Shouldn't be called externally.
    fn push_cost_locals(&mut self, n: usize);

    /// Resets the locals in the cost tracking context.
    ///
    /// Meant to be called before processing the body of a function.
    fn reset_cost_locals(&mut self) {
        self.cost_context().locals = Vec::new();
    }

    /// Executes the given closure with `N` locals used for cost tracking, if
    /// code should be emitted. All locals are of [`ValType::I32`].
    ///
    /// Locals are reused between runs. If the cost tracking context doesn't
    /// have at least `N` locals stored, more will be created using
    /// `push_cost_locals`.
    ///
    /// The locals in the cost context can be emptied using
    /// [`reset_cost_locals`].
    fn with_cost_locals<const N: usize>(&mut self, closure: impl FnOnce(&mut Self, [LocalId; N])) {
        let context = self.cost_context();
        if context.emit {
            let n_locals = context.locals.len();

            if N > n_locals {
                let n_new_locals = N - n_locals;
                self.push_cost_locals(n_new_locals);
            }

            let context = self.cost_context();

            // SAFETY: we can be sure that there are at least N elements in the
            //         vector, so copying an N-sized array's worth of elements is
            //         safe - especially since `LocalId` implements `Copy`.
            let locals = unsafe { ptr::read(context.locals.as_ptr() as _) };
            closure(self, locals);
        }
    }

    /// Executes the given closure with the cost tracking context, if code should be emitted.
    fn with_emit_context(&mut self, closure: impl FnOnce(&CostTrackingContext)) {
        let context = self.cost_context();
        if context.emit {
            let context = self.cost_context();
            closure(context);
        }
    }

    // simple variadic words

    fn cost_add(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 11, 125);
        });
    }

    fn cost_sub(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 11, 125);
        });
    }

    fn cost_mul(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 13, 125);
        });
    }

    fn cost_div(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 13, 125);
        });
    }

    // simple words

    fn cost_log2(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 133);
        });
    }

    fn cost_mod(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 141);
        });
    }

    fn cost_pow(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 143);
        });
    }

    fn cost_sqrti(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 142);
        });
    }

    fn cost_bitwise_and(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 15, 129);
        });
    }

    fn cost_bitwise_or(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 15, 129);
        });
    }

    fn cost_bitwise_xor(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 15, 129);
        });
    }

    fn cost_bitwise_not(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 147);
        });
    }

    fn cost_bitwise_lshift(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 167);
        });
    }

    fn cost_bitwise_rshift(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 167);
        });
    }

    fn cost_buff_to_int_le(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 141);
        });
    }

    fn cost_buff_to_int_be(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 141);
        });
    }

    fn cost_buff_to_uint_le(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 141);
        });
    }

    fn cost_buff_to_uint_be(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 141);
        });
    }

    fn cost_cmp_gt(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 7, 128);
        });
    }

    fn cost_cmp_ge(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 7, 128);
        });
    }

    fn cost_cmp_lt(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 7, 128);
        });
    }

    fn cost_cmp_le(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 7, 128);
        });
    }

    fn cost_or(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 3, 120);
        });
    }

    fn cost_and(&mut self, instrs: &mut InstrSeqBuilder, n: u32) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 3, 120);
        });
    }

    fn cost_int_to_ascii(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 147);
        });
    }

    fn cost_int_to_utf8(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 181);
        });
    }

    fn cost_string_to_int(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 168);
        });
    }

    fn cost_string_to_uint(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 168);
        });
    }

    fn cost_hash160(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 188);
        });
    }

    fn cost_keccak256(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 127);
        });
    }

    fn cost_sha256(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 100);
        });
    }

    fn cost_sha512(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 176);
        });
    }

    fn cost_sha512_256(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 56);
        });
    }

    fn cost_not(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 138);
        });
    }

    fn cost_to_int(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 135);
        });
    }

    fn cost_to_uint(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 135);
        });
    }

    fn cost_destruct(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 314);
        });
    }

    fn cost_is_standard(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 127);
        });
    }

    fn cost_stx_burn(&mut self, _instrs: &mut InstrSeqBuilder) {
        // TODO: check if this indeed costs nothing (SUSPICIOUS)
    }

    fn cost_stx_get_account(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 4654);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_stx_get_balance(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 4294);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    // complex words

    fn cost_let(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 117, 178);
        });
    }

    fn cost_at_block(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 1327);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_get_block_info(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 6321);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_get_burn_block_info(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 96479);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_get_stacks_block_info(&mut self, instrs: &mut InstrSeqBuilder) {
        // TODO: check if this indeed costs the same as `get_block_info`
        self.cost_get_block_info(instrs)
    }

    fn cost_get_tenure_info(&mut self, _instrs: &mut InstrSeqBuilder) {
        // TODO: check if this indeed costs nothing (SUSPICIOUS)
    }

    fn cost_asserts(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 128);
        });
    }

    fn cost_filter(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 407);
        });
    }

    fn cost_if(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 168);
        });
    }

    fn cost_match(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 264);
        });
    }

    fn cost_try(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 240);
        });
    }

    fn cost_unwrap(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 252);
        });
    }

    fn cost_unwrap_err(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 248);
        });
    }

    fn cost_from_consensus_buff(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_nlogn(instrs, CostType::Runtime, n, 3, 185);
        });
    }

    fn cost_to_consensus_buff(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 233);
        });
    }

    fn cost_as_contract(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 138);
        });
    }

    fn cost_contract_call(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 134);
        });
    }

    fn cost_begin(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 151);
        });
    }

    fn cost_unwrap_err_panic(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 302);
        });
    }

    fn cost_unwrap_panic(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 274);
        });
    }

    fn cost_get_data_var(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        let n = n.into();

        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 468);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_linear(instrs, CostType::Runtime, n, 1, 1);
        });
    }

    fn cost_set_data_var(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        let n = n.into();

        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 5, 655);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_linear(instrs, CostType::WriteLength, n, 1, 1);
        });
    }

    fn cost_default_to(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 268);
        });
    }

    fn cost_clarity_err(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 199);
        });
    }

    fn cost_clarity_ok(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 199);
        });
    }

    fn cost_clarity_some(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 199);
        });
    }

    fn cost_index_of(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 211);
        });
    }

    fn cost_is_eq(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 151);
        });
    }

    fn cost_map_get(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        let n = n.into();

        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 1025);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_linear(instrs, CostType::ReadLength, n, 1, 1);
        });
    }

    fn cost_map_set(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        let n = n.into();

        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 4, 1899);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_linear(instrs, CostType::WriteLength, n, 1, 1);
        });
    }

    fn cost_map_insert(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        // TODO: check if this indeed costs the same as `set`
        self.cost_map_set(instrs, n)
    }

    fn cost_map_delete(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        // TODO: check if this indeed costs the same as `set`
        self.cost_map_set(instrs, n)
    }

    fn cost_contract_of(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 13400);
        });
    }

    fn cost_is_none(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 214);
        });
    }

    fn cost_is_some(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 195);
        });
    }

    fn cost_construct(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 398);
        });
    }

    fn cost_principal_of(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 984);
        });
    }

    fn cost_print(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 15, 1458);
        });
    }

    fn cost_is_err(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 245);
        });
    }

    fn cost_is_ok(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 258);
        });
    }

    fn cost_recover(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 8655);
        });
    }

    fn cost_verify(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 8349);
        });
    }

    fn cost_append(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 73, 285);
        });
    }

    fn cost_as_max_len(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 475);
        });
    }

    fn cost_concat(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 37, 220);
        });
    }

    fn cost_element_at(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 498);
        });
    }

    fn cost_fold(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 460);
        });
    }

    fn cost_len(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 429);
        });
    }

    fn cost_list_cons(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 14, 164);
        });
    }

    fn cost_map(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1198, 3067);
        });
    }

    fn cost_replace_at(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 1, 561);
        });
    }

    fn cost_slice(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 498);
        });
    }

    fn cost_stx_transfer(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 4640);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_stx_transfer_memo(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 4709);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_ft_mint(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 1479);
            context.caf_const(instrs, CostType::ReadCount, 2);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 2);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_ft_burn(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 549);
            context.caf_const(instrs, CostType::ReadCount, 2);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 2);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_ft_transfer(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 549);
            context.caf_const(instrs, CostType::ReadCount, 2);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 2);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_ft_get_supply(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 420);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_ft_get_balance(&mut self, instrs: &mut InstrSeqBuilder) {
        self.with_emit_context(|context| {
            context.caf_const(instrs, CostType::Runtime, 479);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_nft_mint(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 9, 575);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_nft_burn(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 9, 572);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_nft_transfer(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 9, 572);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
            context.caf_const(instrs, CostType::WriteCount, 1);
            context.caf_const(instrs, CostType::WriteLength, 1);
        });
    }

    fn cost_nft_get_owner(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 9, 795);
            context.caf_const(instrs, CostType::ReadCount, 1);
            context.caf_const(instrs, CostType::ReadLength, 1);
        });
    }

    fn cost_tuple_get(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_nlogn(instrs, CostType::Runtime, n, 4, 1736);
        });
    }

    fn cost_tuple_merge(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_linear(instrs, CostType::Runtime, n, 4, 408);
        });
    }

    fn cost_tuple_cons(&mut self, instrs: &mut InstrSeqBuilder, n: impl Into<Scalar>) {
        self.with_emit_context(|context| {
            context.caf_nlogn(instrs, CostType::Runtime, n, 10, 1876);
        });
    }

    // TODO: check if these are indeed only relevant during analysis
    // DefineConstant
    // DefineDataVar
    // DefinePrivateFunction
    // DefinePublicFunction
    // DefineReadOnlyFunction
    // DefineFungibleToken
    // DefineNonFungibleToken
    // DefineTrait
}

impl CostTrackingGenerator for WasmGenerator {
    fn cost_context(&mut self) -> &mut CostTrackingContext {
        &mut self.cost_context
    }

    fn push_cost_locals(&mut self, n: usize) {
        for _ in 0..n {
            let local = self.module.locals.add(ValType::I32);
            self.cost_context.locals.push(local);
        }
    }
}

/// A 32-bit unsigned integer to be resolved at either compile-time or run-time.
#[derive(Clone, Copy)]
pub enum Scalar {
    Compile(u32),
    Run(LocalId),
}

impl From<u32> for Scalar {
    fn from(n: u32) -> Self {
        Self::Compile(n)
    }
}

/// Trait for allowing us to not repeat ourselves in resolving a scalar.
trait ScalarGet {
    fn scalar_get(&mut self, scalar: Scalar) -> &mut Self;
}

impl ScalarGet for InstrSeqBuilder<'_> {
    fn scalar_get(&mut self, scalar: Scalar) -> &mut Self {
        match scalar {
            Scalar::Compile(c) => self.i32_const(c as _),
            Scalar::Run(l) => self.local_get(l),
        }
        .instr(Instr::Unop(Unop {
            op: UnaryOp::I64ExtendUI32,
        }))
    }
}

// NOTE: Can we find a way to guarantee that the local is `ValType::I32`?
//       Perhaps it would be possible to do a check with `WasmGenerator`?
impl From<LocalId> for Scalar {
    fn from(n: LocalId) -> Self {
        Self::Run(n)
    }
}

/// Context required from a generator to emit cost tracking code.
pub struct CostTrackingContext {
    /// Whether or not to emit cost tracking code
    pub emit: bool,

    // costs being tracked
    pub runtime: GlobalId,
    pub read_count: GlobalId,
    pub read_length: GlobalId,
    pub write_count: GlobalId,
    pub write_length: GlobalId,

    /// Runtime error function
    pub runtime_error: FunctionId,

    /// Locals used for cost tracking
    pub locals: Vec<LocalId>,
}

enum CostType {
    Runtime,
    ReadCount,
    ReadLength,
    WriteCount,
    WriteLength,
}

impl CostTrackingContext {
    fn global_and_err_code(&self, cost_type: CostType) -> (GlobalId, i32) {
        match cost_type {
            CostType::Runtime => (self.runtime, ErrorMap::CostOverrunRuntime as _),
            CostType::ReadCount => (self.read_count, ErrorMap::CostOverrunReadCount as _),
            CostType::ReadLength => (self.read_length, ErrorMap::CostOverrunReadLength as _),
            CostType::WriteCount => (self.write_count, ErrorMap::CostOverrunWriteCount as _),
            CostType::WriteLength => (self.write_length, ErrorMap::CostOverrunWriteLength as _),
        }
    }

    fn caf_const(
        &self,
        instrs: &mut InstrSeqBuilder,
        cost_type: CostType,
        cost: impl Into<Scalar>,
    ) {
        let cost = cost.into();
        let (global, err_code) = self.global_and_err_code(cost_type);

        instrs.global_get(global).scalar_get(cost);

        // global - cost
        instrs
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Sub,
            }))
            .global_set(global)
            .global_get(global)
            .i64_const(0)
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64LtS,
            }))
            .if_else(
                None,
                |builder| {
                    builder.i32_const(err_code);
                    builder.call(self.runtime_error);
                },
                |_| {},
            );
    }

    fn caf_linear(
        &self,
        instrs: &mut InstrSeqBuilder,
        cost_type: CostType,
        n: impl Into<Scalar>,
        a: u64,
        b: u64,
    ) {
        let n = n.into();
        let (global, err_code) = self.global_and_err_code(cost_type);

        // cost = (+ (* a n) b))
        instrs
            .global_get(global)
            .i64_const(b as _)
            .i64_const(a as _)
            .scalar_get(n)
            // * a
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Mul,
            }))
            // + b
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Add,
            }));

        // global - cost
        instrs
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Sub,
            }))
            .global_set(global)
            .global_get(global)
            .i64_const(0)
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64LtS,
            }))
            .if_else(
                None,
                |builder| {
                    builder.i32_const(err_code);
                    builder.call(self.runtime_error);
                },
                |_| {},
            );
    }

    // NOTE: this seems to only be used during analysis, and since we're only measuring runtime
    //       costs it is unused
    #[allow(unused)]
    fn caf_logn(
        &self,
        instrs: &mut InstrSeqBuilder,
        cost_type: CostType,
        n: impl Into<Scalar>,
        a: u64,
        b: u64,
    ) {
        let n = n.into();
        let (global, err_code) = self.global_and_err_code(cost_type);

        // cost = (+ (* a (log2 n)) b))
        instrs
            .global_get(global)
            .i64_const(b as _)
            .i64_const(a as _)
            .i64_const(63)
            .scalar_get(n)
            // begin log2(n)
            // 63 minus leading zeros in `n`
            // n *must* be larger than 0
            .instr(Instr::Unop(Unop {
                op: UnaryOp::I64Clz,
            }))
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Sub,
            }))
            // * a
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Mul,
            }))
            // + b
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Add,
            }));

        // global - cost
        instrs
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Sub,
            }))
            .global_set(global)
            .global_get(global)
            .i64_const(0)
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64LtS,
            }))
            .if_else(
                None,
                |builder| {
                    builder.i32_const(err_code);
                    builder.call(self.runtime_error);
                },
                |_| {},
            );
    }

    fn caf_nlogn(
        &self,
        instrs: &mut InstrSeqBuilder,
        cost_type: CostType,
        n: impl Into<Scalar>,
        a: u64,
        b: u64,
    ) {
        let n = n.into();
        let (global, err_code) = self.global_and_err_code(cost_type);

        // cost = (+ (* a (* n (log2 n))) b))
        instrs
            .global_get(global)
            .i64_const(b as _)
            .i64_const(a as _)
            .scalar_get(n)
            .i64_const(63)
            .scalar_get(n)
            // log2(n)
            // 63 minus leading zeros in `n`
            // n *must* be larger than 0
            .instr(Instr::Unop(Unop {
                op: UnaryOp::I64Clz,
            }))
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Sub,
            }))
            // * n
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Mul,
            }))
            // * a
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Mul,
            }))
            // + b
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Add,
            }));

        // global - cost
        instrs
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64Sub,
            }))
            .global_set(global)
            .global_get(global)
            .i64_const(0)
            .instr(Instr::Binop(Binop {
                op: BinaryOp::I64LtS,
            }))
            .if_else(
                None,
                |builder| {
                    builder.i32_const(err_code);
                    builder.call(self.runtime_error);
                },
                |_| {},
            );
    }
}
