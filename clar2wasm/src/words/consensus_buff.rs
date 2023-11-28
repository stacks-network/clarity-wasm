use clarity::vm::types::MAX_VALUE_SIZE;
use walrus::ir::InstrSeqType;

use crate::wasm_generator::ArgumentsExt;

use super::Word;

#[derive(Debug)]
pub struct ToConsensusBuf;

impl Word for ToConsensusBuf {
    fn name(&self) -> clarity::vm::ClarityName {
        "to-consensus-buff?".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &clarity::vm::SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let ty = generator
            .get_expr_type(args.get_expr(0)?)
            .expect("to-consensus-buff? value exprission must be typed")
            .clone();

        let offset = generator.module.locals.add(walrus::ValType::I32);
        let length = generator.module.locals.add(walrus::ValType::I32);

        builder
            .global_get(generator.stack_pointer)
            .local_set(offset);

        generator.traverse_args(builder, args)?;

        generator.serialize_to_memory(builder, offset, 0, &ty)?;

        builder.local_set(length);

        builder
            .local_get(length)
            .i32_const(MAX_VALUE_SIZE as i32)
            .binop(walrus::ir::BinaryOp::I32LeU)
            .if_else(
                InstrSeqType::new(
                    &mut generator.module.types,
                    &[],
                    &[
                        walrus::ValType::I32,
                        walrus::ValType::I32,
                        walrus::ValType::I32,
                    ],
                ),
                |then| {
                    then.local_get(offset)
                        .local_get(length)
                        .binop(walrus::ir::BinaryOp::I32Add)
                        .global_set(generator.stack_pointer);

                    then.i32_const(1).local_get(offset).local_get(length);
                },
                |else_| {
                    else_.i32_const(0).i32_const(0).i32_const(0);
                },
            );

        Ok(())
    }
}
