use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::{ComplexWord, SimpleWord};
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct StxBurn;

impl SimpleWord for StxBurn {
    fn name(&self) -> ClarityName {
        "stx-burn?".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // Amount and sender are on the stack, so just call the host interface
        // function, `stx_burn`
        builder.call(generator.func_by_name("stdlib.stx_burn"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct StxGetBalance;

impl SimpleWord for StxGetBalance {
    fn name(&self) -> ClarityName {
        "stx-get-balance".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        builder.call(generator.func_by_name("stdlib.stx_get_balance"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct StxTransfer;

impl ComplexWord for StxTransfer {
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
        builder.call(generator.func_by_name("stdlib.stx_transfer"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct StxTransferMemo;

impl ComplexWord for StxTransferMemo {
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

        builder.call(generator.func_by_name("stdlib.stx_transfer"));
        Ok(())
    }
}

#[derive(Debug)]
pub struct StxGetAccount;

impl SimpleWord for StxGetAccount {
    fn name(&self) -> ClarityName {
        "stx-account".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        builder.call(generator.func_by_name("stdlib.stx_account"));
        Ok(())
    }
}
