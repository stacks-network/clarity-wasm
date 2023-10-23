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
pub struct BurnFungibleToken;

impl Word for BurnFungibleToken {
    fn name(&self) -> ClarityName {
        "ft-burn?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let token = args.get_name(0)?;
        let amount = args.get_expr(1)?;
        let sender = args.get_expr(2)?;

        // Push the token name onto the stack
        let (id_offset, id_length) = generator.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the amount and sender onto the stack
        generator.traverse_expr(builder, amount)?;
        generator.traverse_expr(builder, sender)?;

        // Call the host interface function `ft_burn`
        builder.call(generator.func_by_name("ft_burn"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct TransferFungibleToken;

impl Word for TransferFungibleToken {
    fn name(&self) -> ClarityName {
        "ft-transfer?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let token = args.get_name(0)?;
        let amount = args.get_expr(1)?;
        let sender = args.get_expr(2)?;
        let recipient = args.get_expr(3)?;

        // Push the token name onto the stack
        let (id_offset, id_length) = generator.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the amount, sender, and recipient onto the stack
        generator.traverse_expr(builder, amount)?;
        generator.traverse_expr(builder, sender)?;
        generator.traverse_expr(builder, recipient)?;

        // Call the host interface function `ft_transfer`
        builder.call(generator.func_by_name("ft_transfer"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct MintFungibleToken;

impl Word for MintFungibleToken {
    fn name(&self) -> ClarityName {
        "ft-mint?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let token = args.get_name(0)?;
        let amount = args.get_expr(1)?;
        let recipient = args.get_expr(2)?;

        let (id_offset, id_length) = generator.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the amount and recipient onto the stack
        generator.traverse_expr(builder, amount)?;
        generator.traverse_expr(builder, recipient)?;

        // Call the host interface function `ft_mint`
        builder.call(generator.func_by_name("ft_mint"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct GetSupplyOfFungibleToken;

impl Word for GetSupplyOfFungibleToken {
    fn name(&self) -> ClarityName {
        "ft-get-supply".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let token = args.get_name(0)?;

        let (id_offset, id_length) = generator.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        builder.call(generator.func_by_name("ft_get_supply"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct GetBalanceOfFungibleToken;

impl Word for GetBalanceOfFungibleToken {
    fn name(&self) -> ClarityName {
        "ft-get-balance".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let token = args.get_name(0)?;
        let owner = args.get_expr(1)?;

        // Push the token name onto the stack
        let (id_offset, id_length) = generator.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the owner onto the stack
        generator.traverse_expr(builder, owner)?;

        // Call the host interface function `ft_get_balance`
        builder.call(generator.func_by_name("ft_get_balance"));

        Ok(())
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

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
