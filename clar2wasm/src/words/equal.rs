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
        let name = self.name();

        generator.traverse_args(builder, args)?;

        let ty = generator
            .get_expr_type(&args[0])
            .expect("comparison operands must be typed");

        let type_suffix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                "string-ascii"
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                "string-utf8"
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => "buffer",
            _ => {
                return Err(GeneratorError::InternalError(
                    "invalid type for comparison".to_string(),
                ))
            }
        };

        let func = generator
            .module
            .funcs
            .by_name(&format!("{name}-{type_suffix}"))
            .unwrap_or_else(|| panic!("function not found: {name}-{type_suffix}"));

        let mut results = Vec::new();
        generator.traverse_expr(builder, &args[0])?;
        generator.traverse_expr(builder, &args[1])?;
        builder.call(func);

        let result = generator.module.locals.add(ValType::I32);
        builder.local_set(result);
        results.push(result);

        // Push operands, starting from 3rd, onto the stack
        for operand in args.iter().skip(2) {
            generator.traverse_expr(builder, &args[0])?;
            generator.traverse_expr(builder, operand)?;
            builder.call(func);

            let result = generator.module.locals.add(ValType::I32);
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
