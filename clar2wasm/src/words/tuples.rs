use std::collections::HashMap;

use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};

use crate::wasm_generator::GeneratorError;

use super::Word;

#[derive(Debug)]
pub struct TupleCons;

impl Word for TupleCons {
    fn name(&self) -> ClarityName {
        "tuple".into()
    }

    fn traverse(
        &self,
        generator: &mut crate::wasm_generator::WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[clarity::vm::SymbolicExpression],
    ) -> Result<(), crate::wasm_generator::GeneratorError> {
        let ty = generator
            .get_expr_type(expr)
            .ok_or(GeneratorError::InternalError(
                "tuple expression must be typed".to_string(),
            ))?
            .clone();

        let tuple_ty = match ty {
            TypeSignature::TupleType(tuple) => tuple,
            _ => {
                return Err(GeneratorError::InternalError(
                    "expected tuple type".to_string(),
                ))
            }
        };

        // The args for `tuple` should be pairs of values, with the first value
        // being the key and the second being the value. We need to arrange the
        // values in the correct order for the tuple type, so we'll build a map
        // of the keys to their values.
        let mut values = HashMap::new();
        for arg in args {
            let list = arg.match_list().ok_or(GeneratorError::InternalError(
                "expected key-value pairs in tuple".to_string(),
            ))?;
            if list.len() != 2 {
                return Err(GeneratorError::InternalError(
                    "expected key-value pairs in tuple".to_string(),
                ));
            }

            let key = list[0].match_atom().ok_or(GeneratorError::InternalError(
                "expected key-value pairs in tuple".to_string(),
            ))?;
            values.insert(key, &list[1]);
        }

        // Now we can iterate over the tuple type and build the tuple.
        for key in tuple_ty.get_type_map().keys() {
            let value = values
                .remove(key)
                .ok_or(GeneratorError::InternalError(format!(
                    "missing key '{key}' in tuple"
                )))?;
            generator.traverse_expr(builder, value)?;
        }

        Ok(())
    }
}
