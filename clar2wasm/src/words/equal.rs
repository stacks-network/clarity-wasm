use crate::wasm_generator::{clar2wasm_ty, ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression, types::TypeSignature, types::SequenceSubtype, types::StringSubtype};
use walrus::ValType;

use super::Word;

#[derive(Debug)]
pub struct Equal;

impl Word for Equal {
    fn name(&self) -> ClarityName {
        "is-eq".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let mut results = Vec::new();

        // Traverse the first two values, leaving them on the stack
        let first_op = args.get_expr(0)?;
        generator.traverse_expr(builder, first_op)?;
        let sec_op = args.get_expr(1)?;
        generator.traverse_expr(builder, sec_op)?;

        builder.call(generator.func_by_name("is-eq-int"));

        let mut result = generator.module.locals.add(ValType::I32);
        builder.local_set(result);
        results.push(result);

        // Push first and other operands, starting from 3rd, onto the stack
        for operand in args.iter().skip(2) {
            generator.traverse_expr(builder, first_op)?;
            generator.traverse_expr(builder, operand)?;
            builder.call(generator.func_by_name("is-eq-int"));

            result = generator.module.locals.add(ValType::I32);
            builder.local_set(result);
            results.push(result);
        }

        for result in &results {
            println!("{:?}", generator.module.locals.get(*result));
        }

        builder.local_get(result);

        Ok(())
    }
}
