use super::super::STDLIB_PREFIX;
use super::Word;

fn traverse_buffer_to_integer(
    name: &str,
    generator: &mut crate::wasm_generator::WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[clarity::vm::SymbolicExpression],
) -> Result<(), crate::wasm_generator::GeneratorError> {
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
        traverse_buffer_to_integer(
            &format!("{STDLIB_PREFIX}.buff-to-uint-be"),
            generator,
            builder,
            args,
        )
    }
}

#[derive(Debug)]
pub struct BuffToIntBe;

impl Word for BuffToIntBe {
    fn name(&self) -> clarity::vm::ClarityName {
        "buff-to-int-be".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        // This is the same function as "buff-to-uint-be", with the result interpreted
        // as i128 instead of u128.
        traverse_buffer_to_integer(
            &format!("{STDLIB_PREFIX}.buff-to-uint-be"),
            generator,
            builder,
            args,
        )
    }
}

#[derive(Debug)]
pub struct BuffToUintLe;

impl Word for BuffToUintLe {
    fn name(&self) -> clarity::vm::ClarityName {
        "buff-to-uint-le".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        traverse_buffer_to_integer(
            &format!("{STDLIB_PREFIX}.buff-to-uint-le"),
            generator,
            builder,
            args,
        )
    }
}

#[derive(Debug)]
pub struct BuffToIntLe;

impl Word for BuffToIntLe {
    fn name(&self) -> clarity::vm::ClarityName {
        "buff-to-int-le".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        // This is the same function as "buff-to-uint-le", with the result interpreted
        // as i128 instead of u128.
        traverse_buffer_to_integer(
            &format!("{STDLIB_PREFIX}.buff-to-uint-le"),
            generator,
            builder,
            args,
        )
    }
}
