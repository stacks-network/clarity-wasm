use clarity::vm::types::TypeSignature;
use clarity::vm::ClarityName;

use super::SimpleWord;
use crate::{
    cost::CostTrackingGenerator,
    wasm_generator::{GeneratorError, WasmGenerator},
};

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
        generator.cost_not(builder);
        builder.call(generator.func_by_name("stdlib.not"));
        Ok(())
    }
}
