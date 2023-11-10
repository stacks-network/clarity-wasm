use super::Word;

#[derive(Debug)]
pub struct StringToInt;

impl Word for StringToInt {
    fn name(&self) -> clarity::vm::ClarityName {
        "string-to-int?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_args(builder, args)?;

        let func = generator.func_by_name("stdlib.string-to-int");
        builder.call(func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct StringToUint;

impl Word for StringToUint {
    fn name(&self) -> clarity::vm::ClarityName {
        "string-to-uint?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        generator.traverse_args(builder, args)?;

        let func = generator.func_by_name("stdlib.string-to-uint");
        builder.call(func);

        Ok(())
    }
}
