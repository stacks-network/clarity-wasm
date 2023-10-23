use crate::wasm_generator::ArgumentsExt;
use clarity::vm::{
    clarity_wasm::get_type_in_memory_size, ClarityName, SymbolicExpression, SymbolicExpressionType,
};
use walrus::ValType;

use super::Word;

#[derive(Debug)]
pub struct DefineConstant;

impl Word for DefineConstant {
    fn name(&self) -> ClarityName {
        "define-constant".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let name = args.get_name(0)?;
        let value = args.get_expr(1)?;

        // If the initial value is a literal, then we can directly add it to
        // the literal memory.
        let offset = if let SymbolicExpressionType::LiteralValue(value) = &value.expr {
            let (offset, _len) = generator.add_literal(value);
            offset
        } else {
            // If the initial expression is not a literal, then we need to
            // reserve the space for it, and then execute the expression and
            // write the result into the reserved space.
            let offset = generator.literal_memory_end;
            let offset_local = generator.module.locals.add(ValType::I32);
            builder.i32_const(offset as i32).local_set(offset_local);

            let ty = generator
                .get_expr_type(value)
                .expect("constant value must be typed")
                .clone();

            let len = get_type_in_memory_size(&ty, true) as u32;
            generator.literal_memory_end += len;

            // Traverse the initial value expression.
            generator.traverse_expr(builder, value)?;

            // Write the initial value to the memory, to be read by the host.
            generator.write_to_memory(builder, offset_local, 0, &ty);

            offset
        };

        generator.constants.insert(name.to_string(), offset);

        Ok(())
    }
}
