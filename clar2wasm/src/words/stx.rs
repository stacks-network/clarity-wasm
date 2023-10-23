use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct StxBurn;

impl Word for StxBurn {
    fn name(&self) -> ClarityName {
        "stx-burn?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let amount = args.get_expr(0)?;
        let sender = args.get_expr(1)?;

        generator.traverse_expr(builder, amount)?;
        generator.traverse_expr(builder, sender)?;

        // Amount and sender are on the stack, so just call the host interface
        // function, `stx_burn`
        builder.call(generator.func_by_name("stx_burn"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct StxGetBalance;

impl Word for StxGetBalance {
    fn name(&self) -> ClarityName {
        "stx-get-balance".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let owner = args.get_expr(0)?;
        generator.traverse_expr(builder, owner)?;
        builder.call(generator.func_by_name("stx_get_balance"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct StxTransfer;

impl Word for StxTransfer {
    fn name(&self) -> ClarityName {
        "stx-transfer?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let amount = args.get_expr(0)?;
        let sender = args.get_expr(1)?;
        let recipient = args.get_expr(2)?;

        generator.traverse_expr(builder, amount)?;
        generator.traverse_expr(builder, sender)?;
        generator.traverse_expr(builder, recipient)?;

        // placeholder for memo
        builder.i32_const(0).i32_const(0);
        builder.call(generator.func_by_name("stx_transfer"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct StxTransferMemo;

impl Word for StxTransferMemo {
    fn name(&self) -> ClarityName {
        "stx-transfer-memo?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let amount = args.get_expr(0)?;
        let sender = args.get_expr(1)?;
        let recipient = args.get_expr(2)?;
        let memo = args.get_expr(3)?;

        generator.traverse_expr(builder, amount)?;
        generator.traverse_expr(builder, sender)?;
        generator.traverse_expr(builder, recipient)?;
        generator.traverse_expr(builder, memo)?;

        builder.call(generator.func_by_name("stx_transfer"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct StxGetAccount;

impl Word for StxGetAccount {
    fn name(&self) -> ClarityName {
        "stx-account".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        generator.traverse_args(builder, &args[0..1])?;
        builder.call(generator.func_by_name("stx_account"));
        Ok(())
    }
}
