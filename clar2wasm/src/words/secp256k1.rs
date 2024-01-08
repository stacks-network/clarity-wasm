use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct Recover;

impl ComplexWord for Recover {
    fn name(&self) -> ClarityName {
        "secp256k1-recover?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        generator.traverse_expr(builder, args.get_expr(0)?)?;
        generator.traverse_expr(builder, args.get_expr(1)?)?;

        // Reserve stack space for the host-function to write the result
        let ret_ty = generator
            .get_expr_type(expr)
            .ok_or(GeneratorError::TypeError(
                "result of secp256k1-recover? should be typed".to_owned(),
            ))?
            .clone();

        let (result_local, result_size) =
            generator.create_call_stack_local(builder, &ret_ty, true, true);
        builder.local_get(result_local).i32_const(result_size);

        // Call the host interface function, `secp256k1_recover`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.secp256k1_recover")
                .ok_or(GeneratorError::InternalError(
                    "stdlib.secp256k1_recover not found".to_owned(),
                ))?,
        );

        generator.read_from_memory(builder, result_local, 0, &ret_ty);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Verify;

impl ComplexWord for Verify {
    fn name(&self) -> ClarityName {
        "secp256k1-verify".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        generator.traverse_expr(builder, args.get_expr(0)?)?;
        generator.traverse_expr(builder, args.get_expr(1)?)?;
        generator.traverse_expr(builder, args.get_expr(2)?)?;

        // Call the host interface function, `secp256k1_verify`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.secp256k1_verify")
                .ok_or(GeneratorError::InternalError(
                    "stdlib.secp256k1_verify not found".to_owned(),
                ))?,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::evaluate;

    #[test]
    fn test_secp256k1_recover() {
        let mut expected = [0u8; 33];
        hex::decode_to_slice(
            "03adb8de4bfb65db2cfd6120d55c6526ae9c52e675db7e47308636534ba7786110",
            &mut expected,
        )
        .unwrap();
        assert_eq!(
            evaluate("(secp256k1-recover? 0xde5b9eb9e7c5592930eb2e30a01369c36586d872082ed8181ee83d2a0ec20f04
                0x8738487ebe69b93d8e51583be8eee50bb4213fc49c767d329632730cc193b873554428fc936ca3569afc15f1c9365f6591d6251a89fee9c9ac661116824d3a1301)"),
            Some(Value::okay(Value::buff_from(expected.to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    fn test_secp256k1_verify() {
        assert_eq!(evaluate("(secp256k1-verify 0xde5b9eb9e7c5592930eb2e30a01369c36586d872082ed8181ee83d2a0ec20f04
            0x8738487ebe69b93d8e51583be8eee50bb4213fc49c767d329632730cc193b873554428fc936ca3569afc15f1c9365f6591d6251a89fee9c9ac661116824d3a1301
            0x03adb8de4bfb65db2cfd6120d55c6526ae9c52e675db7e47308636534ba7786110)"), Some(Value::Bool(true)));
        assert_eq!(evaluate("(secp256k1-verify 0xde5b9eb9e7c5592930eb2e30a01369c36586d872082ed8181ee83d2a0ec20f04
            0x8738487ebe69b93d8e51583be8eee50bb4213fc49c767d329632730cc193b873554428fc936ca3569afc15f1c9365f6591d6251a89fee9c9ac661116824d3a13
            0x03adb8de4bfb65db2cfd6120d55c6526ae9c52e675db7e47308636534ba7786110)"), Some(Value::Bool(true)));
        assert_eq!(evaluate("(secp256k1-verify 0x0000000000000000000000000000000000000000000000000000000000000000
            0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
            0x03adb8de4bfb65db2cfd6120d55c6526ae9c52e675db7e47308636534ba7786110)"), Some(Value::Bool(false)));
    }
}
