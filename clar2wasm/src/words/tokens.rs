use crate::wasm_generator::ArgumentsExt;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct DefineFungibleToken;

impl Word for DefineFungibleToken {
    fn name(&self) -> ClarityName {
        "define-fungible-token".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let name = args.get_name(0)?;
        let supply = args.get(1);

        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = generator.add_identifier_string_literal(name);

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        // Push the supply to the stack, as an optional uint
        // (first i32 indicates some/none)
        if let Some(supply) = supply {
            builder.i32_const(1);
            generator.traverse_expr(builder, supply)?;
        } else {
            builder.i32_const(0).i64_const(0).i64_const(0);
        }

        builder.call(generator.func_by_name("define_ft"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefineNonFungibleToken;

impl Word for DefineNonFungibleToken {
    fn name(&self) -> ClarityName {
        "define-non-fungible-token".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let name = args.get_name(0)?;
        let _nft_type = args.get_expr(1)?;

        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = generator.add_identifier_string_literal(name);

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        builder.call(
            generator
                .module
                .funcs
                .by_name("define_nft")
                .expect("function not found"),
        );
        Ok(())
    }
}
