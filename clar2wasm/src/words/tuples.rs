use std::collections::{BTreeMap, HashMap};

use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{clar2wasm_ty, drop_value, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct TupleCons;

impl ComplexWord for TupleCons {
    fn name(&self) -> ClarityName {
        "tuple".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("tuple expression must be typed".to_string()))?
            .clone();

        let tuple_ty = match ty {
            TypeSignature::TupleType(tuple) => tuple,
            _ => return Err(GeneratorError::TypeError("expected tuple type".to_string())),
        };

        // The args for `tuple` should be pairs of values, with the first value
        // being the key and the second being the value. We need to arrange the
        // values in the correct order for the tuple type, so we'll build a map
        // of the keys to their values.
        let mut values = HashMap::new();
        for arg in args {
            let list = arg.match_list().ok_or_else(|| {
                GeneratorError::InternalError("expected key-value pairs in tuple".to_string())
            })?;
            if list.len() != 2 {
                return Err(GeneratorError::InternalError(
                    "expected key-value pairs in tuple".to_string(),
                ));
            }

            let key = list[0].match_atom().ok_or_else(|| {
                GeneratorError::InternalError("expected key-value pairs in tuple".to_string())
            })?;
            values.insert(key, &list[1]);
        }

        // Now we can iterate over the tuple type and build the tuple.
        for (key, ty) in tuple_ty.get_type_map() {
            let value = values.remove(key).ok_or_else(|| {
                GeneratorError::InternalError(format!("missing key '{key}' in tuple"))
            })?;

            // WORKAROUND: if you have a tuple like `(tuple (foo none))`, the `none` will have the type
            // NoType, even if it has a defined type in the tuple. This creates issues because the placeholder
            // does not have the same amount of values in the Wasm code than the correct type.
            // While we wait for a real fix in the typechecker, here is a workaround to make sure that the type
            // is correct.
            generator.set_expr_type(value, ty.clone());

            generator.traverse_expr(builder, value)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct TupleGet;

impl ComplexWord for TupleGet {
    fn name(&self) -> ClarityName {
        "get".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 2 {
            return Err(GeneratorError::InternalError(
                "expected two arguments to tuple get".to_string(),
            ));
        }

        let target_field_name = args[0]
            .match_atom()
            .ok_or_else(|| GeneratorError::InternalError("expected key name".into()))?;

        let tuple_ty = generator
            .get_expr_type(&args[1])
            .ok_or_else(|| GeneratorError::TypeError("tuple expression must be typed".to_string()))
            .and_then(|lhs_ty| match lhs_ty {
                TypeSignature::TupleType(tuple) => Ok(tuple),
                _ => Err(GeneratorError::TypeError("expected tuple type".to_string())),
            })?
            .clone();

        // Traverse the tuple argument, leaving it on top of the stack.
        generator.traverse_expr(builder, &args[1])?;

        // Determine the wasm types for each field of the tuple
        let field_types = tuple_ty.get_type_map();

        // Create locals for the target field
        let wasm_types = clar2wasm_ty(field_types.get(target_field_name).ok_or_else(|| {
            GeneratorError::InternalError(format!("missing field '{target_field_name}' in tuple"))
        })?);
        let mut val_locals = Vec::with_capacity(wasm_types.len());
        for local_ty in wasm_types.iter().rev() {
            let local = generator.module.locals.add(*local_ty);
            val_locals.push(local);
        }

        // Loop through the fields of the tuple, in reverse order. When we find
        // the target field, we'll store it in the locals we created above. All
        // other fields will be dropped.
        for (field_name, field_ty) in field_types.iter().rev() {
            // If this is the target field, store it in the locals we created
            // above.
            if field_name == target_field_name {
                for local in val_locals.iter() {
                    builder.local_set(*local);
                }
            } else {
                drop_value(builder, field_ty);
            }
        }

        // Load the target field from the locals we created above.
        for local in val_locals.iter().rev() {
            builder.local_get(*local);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct TupleMerge;

impl ComplexWord for TupleMerge {
    fn name(&self) -> ClarityName {
        "merge".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 2 {
            return Err(GeneratorError::InternalError(
                "expected two arguments to tuple merge".to_string(),
            ));
        }

        let lhs_tuple_ty = generator
            .get_expr_type(&args[0])
            .ok_or_else(|| GeneratorError::TypeError("tuple expression must be typed".to_string()))
            .and_then(|lhs_ty| match lhs_ty {
                TypeSignature::TupleType(tuple) => Ok(tuple),
                _ => Err(GeneratorError::TypeError("expected tuple type".to_string())),
            })?
            .clone();

        let rhs_tuple_ty = generator
            .get_expr_type(&args[1])
            .ok_or_else(|| GeneratorError::TypeError("tuple expression must be typed".to_string()))
            .and_then(|lhs_ty| match lhs_ty {
                TypeSignature::TupleType(tuple) => Ok(tuple),
                _ => Err(GeneratorError::TypeError("expected tuple type".to_string())),
            })?
            .clone();

        // Traverse the LHS tuple argument, leaving it on top of the stack.
        generator.traverse_expr(builder, &args[0])?;

        // We need to merge the two tuples and then push the combined tuple
        // back onto the stack in the correct order. To do this, we'll store
        // the values of the LHS tuple in locals, and then store the values of
        // the RHS tuple in locals (overwriting LHS values when there are name
        // collisions). Finally, we'll load the values from those locals in the
        // correct order.
        let mut locals = BTreeMap::new();
        // LHS
        for (field_name, field_ty) in lhs_tuple_ty.get_type_map().iter().rev() {
            let field_locals = generator.save_to_locals(builder, field_ty, false);
            locals.insert(field_name, field_locals);
        }

        // Traverse the LHS tuple argument, leaving it on top of the stack.
        generator.traverse_expr(builder, &args[1])?;

        // RHS
        for (field_name, field_ty) in rhs_tuple_ty.get_type_map().iter().rev() {
            let wasm_types = clar2wasm_ty(field_ty);
            let mut field_locals = Vec::with_capacity(wasm_types.len());
            // If this field was in the LHS, then we'll store to the existing
            // locals instead of creating new ones.
            if let Some(field_locals) = locals.get(field_name) {
                for local in field_locals {
                    builder.local_set(*local);
                }
            } else {
                for local_ty in wasm_types.iter().rev() {
                    let local = generator.module.locals.add(*local_ty);
                    builder.local_set(local);
                    field_locals.push(local);
                }
                locals.insert(field_name, field_locals);
            }
        }

        // Now load the combined values from the locals we created above.
        for (_, field_locals) in locals {
            for local in field_locals.iter().rev() {
                builder.local_get(*local);
            }
        }

        Ok(())
    }
}
