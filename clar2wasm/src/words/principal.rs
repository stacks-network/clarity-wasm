use clarity::{
    address::{
        C32_ADDRESS_VERSION_MAINNET_MULTISIG, C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
        C32_ADDRESS_VERSION_TESTNET_MULTISIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
    },
    vm::{ClarityName, SymbolicExpression},
};
use walrus::{
    ir::{BinaryOp, InstrSeqType, MemArg},
    ValType,
};

use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

use super::Word;

#[derive(Debug)]
pub struct IsStandard;

impl Word for IsStandard {
    fn name(&self) -> ClarityName {
        "is-standard".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // Traverse the principal
        generator.traverse_expr(builder, args.get_expr(0)?)?;

        // Drop the length
        builder.drop();

        // Read the version byte from the principal in memory
        builder.load(
            generator.get_memory(),
            walrus::ir::LoadKind::I32_8 {
                kind: walrus::ir::ExtendedLoad::ZeroExtend,
            },
            MemArg {
                align: 1,
                offset: 0,
            },
        );

        // Save the version byte in a local
        let version_local = generator.module.locals.add(ValType::I32);
        builder.local_tee(version_local);

        // TODO: It would be nice if this was a global variable that gets set
        //       at compile time, instead of requiring a host-interface call.
        // Check if we are in mainnet (leaves a boolean on the stack)
        builder.call(generator.func_by_name("is_in_mainnet"));

        builder.if_else(
            InstrSeqType::new(
                &mut generator.module.types,
                &[ValType::I32],
                &[ValType::I32],
            ),
            |then| {
                then.i32_const(C32_ADDRESS_VERSION_MAINNET_MULTISIG as i32)
                    .binop(BinaryOp::I32Eq);
                then.local_get(version_local)
                    .i32_const(C32_ADDRESS_VERSION_MAINNET_SINGLESIG as i32)
                    .binop(BinaryOp::I32Eq);
                then.binop(BinaryOp::I32Or);
            },
            |else_| {
                else_
                    .i32_const(C32_ADDRESS_VERSION_TESTNET_MULTISIG as i32)
                    .binop(BinaryOp::I32Eq);
                else_
                    .local_get(version_local)
                    .i32_const(C32_ADDRESS_VERSION_TESTNET_SINGLESIG as i32)
                    .binop(BinaryOp::I32Eq);
                else_.binop(BinaryOp::I32Or);
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::evaluate;

    #[test]
    fn test_is_standard() {
        assert_eq!(
            evaluate("(is-standard 'STB44HYPYAT2BB2QE513NSP81HTMYWBJP02HPGK6)"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_is_standard_contract() {
        assert_eq!(
            evaluate("(is-standard 'STB44HYPYAT2BB2QE513NSP81HTMYWBJP02HPGK6.foo)"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_is_standard_multisig() {
        assert_eq!(
            evaluate("(is-standard 'SN3X6QWWETNBZWGBK6DRGTR1KX50S74D340JWTSC7)"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_is_standard_mainnet() {
        assert_eq!(
            evaluate("(is-standard 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY)"),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_is_standard_mainnet_contract() {
        assert_eq!(
            evaluate("(is-standard 'SP3X6QWWETNBZWGBK6DRGTR1KX50S74D3433WDGJY.foo)"),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_is_standard_mainnet_multisig() {
        assert_eq!(
            evaluate("(is-standard 'SM3X6QWWETNBZWGBK6DRGTR1KX50S74D341M9C5X7)"),
            Some(Value::Bool(false))
        );
    }

    #[test]
    fn test_is_standard_other() {
        assert_eq!(
            evaluate("(is-standard 'SZ2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQ9H6DPR)"),
            Some(Value::Bool(false))
        );
    }
}
