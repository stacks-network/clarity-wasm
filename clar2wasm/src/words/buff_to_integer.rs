use super::Word;

fn traverse_buffer_to_integer(
    word: &impl Word,
    generator: &mut crate::wasm_generator::WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[clarity::vm::SymbolicExpression],
) -> Result<(), crate::wasm_generator::GeneratorError> {
    let name = &word.name();
    let func = generator
        .module
        .funcs
        .by_name(name)
        .unwrap_or_else(|| panic!("function not found: {name}"));
    generator.traverse_args(builder, args)?;
    builder.call(func);
    Ok(())
}

#[derive(Debug)]
pub struct BuffToUintBe;

impl Word for BuffToUintBe {
    fn name(&self) -> clarity::vm::ClarityName {
        "buff-to-uint-be".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_buffer_to_integer(self, generator, builder, args)
    }
}
