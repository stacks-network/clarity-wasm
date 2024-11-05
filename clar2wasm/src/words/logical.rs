use clarity::vm::types::TypeSignature;
use clarity::vm::ClarityName;

use super::SimpleWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct Not;

impl SimpleWord for Not {
    fn name(&self) -> ClarityName {
        "not".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        builder.call(generator.func_by_name("stdlib.not"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::tools::{crosscheck, evaluate};

    #[test]
    fn test_not_true() {
        crosscheck("(not true)", evaluate("false"));
    }

    #[test]
    fn test_not_false() {
        crosscheck("(not false)", evaluate("true"));
    }

    #[test]
    fn test_not_not_false() {
        crosscheck("(not (not false))", evaluate("false"));
    }

    #[test]
    fn test_not_if_false() {
        crosscheck("(not (if false true false))", evaluate("true"));
    }
}
