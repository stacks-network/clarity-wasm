use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, LiteralMemoryEntry, WasmGenerator};

#[derive(Debug)]
pub struct MapDefinition;

impl ComplexWord for MapDefinition {
    fn name(&self) -> ClarityName {
        "define-map".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        // Making sure if name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {:?}",
                name
            )));
        }

        let key_type = args.get_expr(1).and_then(|sym_ty| {
            TypeSignature::parse_type_repr(generator.contract_analysis.epoch, sym_ty, &mut ())
                .map_err(|e| GeneratorError::TypeError(format!("invalid type for map key: {e}")))
        })?;
        let value_type = args.get_expr(2).and_then(|sym_ty| {
            TypeSignature::parse_type_repr(generator.contract_analysis.epoch, sym_ty, &mut ())
                .map_err(|e| GeneratorError::TypeError(format!("invalid type for map value: {e}")))
        })?;

        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = generator.add_string_literal(name)?;

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.define_map")
                .ok_or_else(|| {
                    GeneratorError::InternalError("stdlib.define_map not found".to_owned())
                })?,
        );

        // Add the map types to generator
        generator
            .maps_types
            .insert(name.clone(), (key_type, value_type));

        Ok(())
    }
}

#[derive(Debug)]
pub struct MapGet;

impl ComplexWord for MapGet {
    fn name(&self) -> ClarityName {
        "map-get?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let key = args.get_expr(1)?;

        // WORKAROUND: set correct type for key
        if let Some((key_ty, _)) = generator.maps_types.get(name) {
            generator.set_expr_type(key, key_ty.clone())?;
        }

        // Get the offset and length for this identifier in the literal memory
        let id_offset = *generator
            .literal_memory_offset
            .get(&LiteralMemoryEntry::Ascii(name.as_str().into()))
            .ok_or_else(|| GeneratorError::InternalError(format!("map not found: {name}")))?;
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = generator
            .get_expr_type(key)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-set value expression must be typed".to_owned())
            })?
            .clone();
        let (key_offset, key_size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Push the key to the data stack
        generator.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        generator.write_to_memory(builder, key_offset, 0, &ty)?;

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Create a new local to hold the result on the call stack
        let ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-get? expression must be typed".to_owned())
            })?
            .clone();
        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &ty, true, true);

        // Push the return value offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host-interface function, `map_get`
        builder.call(generator.func_by_name("stdlib.map_get"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &ty)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct MapSet;

impl ComplexWord for MapSet {
    fn name(&self) -> ClarityName {
        "map-set".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let key = args.get_expr(1)?;
        let value = args.get_expr(2)?;

        // WORKAROUND: set correct types for key and value
        if let Some((key_ty, value_ty)) = generator.maps_types.get(name).cloned() {
            generator.set_expr_type(key, key_ty)?;
            generator.set_expr_type(value, value_ty)?;
        }

        // Get the offset and length for this identifier in the literal memory
        let id_offset = *generator
            .literal_memory_offset
            .get(&LiteralMemoryEntry::Ascii(name.as_str().into()))
            .ok_or_else(|| GeneratorError::InternalError(format!("map not found: {name}")))?;
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = generator
            .get_expr_type(key)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-set value expression must be typed".to_owned())
            })?
            .clone();
        let (key_offset, key_size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Push the key to the data stack
        generator.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        generator.write_to_memory(builder, key_offset, 0, &ty)?;

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Create space on the call stack to write the value
        let ty = generator
            .get_expr_type(value)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-set value expression must be typed".to_owned())
            })?
            .clone();
        let (val_offset, val_size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Push the value to the data stack
        generator.traverse_expr(builder, value)?;

        // Write the value to the memory (it's already on the data stack)
        generator.write_to_memory(builder, val_offset, 0, &ty)?;

        // Push the value offset and size to the data stack
        builder.local_get(val_offset).i32_const(val_size);

        // Call the host interface function, `map_set`
        builder.call(generator.func_by_name("stdlib.map_set"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct MapInsert;

impl ComplexWord for MapInsert {
    fn name(&self) -> ClarityName {
        "map-insert".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let key = args.get_expr(1)?;
        let value = args.get_expr(2)?;

        // WORKAROUND: set correct types for key and value
        if let Some((key_ty, value_ty)) = generator.maps_types.get(name).cloned() {
            generator.set_expr_type(key, key_ty)?;
            generator.set_expr_type(value, value_ty)?;
        }

        // Get the offset and length for this identifier in the literal memory
        let id_offset = *generator
            .literal_memory_offset
            .get(&LiteralMemoryEntry::Ascii(name.as_str().into()))
            .ok_or_else(|| GeneratorError::InternalError(format!("map not found: {name}")))?;
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = generator
            .get_expr_type(key)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-set value expression must be typed".to_owned())
            })?
            .clone();
        let (key_offset, key_size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Push the key to the data stack
        generator.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        generator.write_to_memory(builder, key_offset, 0, &ty)?;

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Create space on the call stack to write the value
        let ty = generator
            .get_expr_type(value)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-set value expression must be typed".to_owned())
            })?
            .clone();
        let (val_offset, val_size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Push the value to the data stack
        generator.traverse_expr(builder, value)?;

        // Write the value to the memory (it's already on the data stack)
        generator.write_to_memory(builder, val_offset, 0, &ty)?;

        // Push the value offset and size to the data stack
        builder.local_get(val_offset).i32_const(val_size);

        // Call the host interface function, `map_insert`
        builder.call(generator.func_by_name("stdlib.map_insert"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct MapDelete;

impl ComplexWord for MapDelete {
    fn name(&self) -> ClarityName {
        "map-delete".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let key = args.get_expr(1)?;

        // WORKAROUND: set correct type for key
        if let Some((key_ty, _)) = generator.maps_types.get(name) {
            generator.set_expr_type(key, key_ty.clone())?;
        }

        // Get the offset and length for this identifier in the literal memory
        let id_offset = *generator
            .literal_memory_offset
            .get(&LiteralMemoryEntry::Ascii(name.as_str().into()))
            .ok_or_else(|| GeneratorError::InternalError(format!("map not found: {name}")))?;

        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = generator
            .get_expr_type(key)
            .ok_or_else(|| {
                GeneratorError::TypeError("map-set value expression must be typed".to_owned())
            })?
            .clone();
        let (key_offset, key_size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Push the key to the data stack
        generator.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        generator.write_to_memory(builder, key_offset, 0, &ty)?;

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Call the host interface function, `map_delete`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.map_delete")
                .ok_or_else(|| {
                    GeneratorError::TypeError("stdlib.map_delete not found".to_owned())
                })?,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::{crosscheck, crosscheck_compare_only};

    #[test]
    fn map_define_get() {
        crosscheck(
            r#"(define-map counters principal uint) (map-get? counters tx-sender)"#,
            Ok(Some(Value::none())),
        )
    }

    #[test]
    fn map_define_set() {
        crosscheck("(define-map approved-contracts principal bool) (map-set approved-contracts tx-sender true)", Ok(Some(Value::Bool(true))));
    }

    #[test]
    fn map_define_insert() {
        crosscheck("(define-map approved-contracts principal bool) (map-insert approved-contracts tx-sender true)", Ok(Some(Value::Bool(true))));
    }

    #[test]
    fn map_define_set_delete() {
        crosscheck("(define-map approved-contracts principal bool) (map-insert approved-contracts tx-sender true) (map-delete approved-contracts tx-sender)", Ok(Some(Value::Bool(true))));
    }

    #[test]
    fn map_define_set_get() {
        crosscheck("(define-map approved-contracts principal bool) (map-insert approved-contracts tx-sender true) (map-get? approved-contracts tx-sender)", Ok(Some(Value::some(Value::Bool(true)).unwrap())));
    }

    #[test]
    fn validate_define_map() {
        // Reserved keyword
        crosscheck_compare_only("(define-map map {x: int} {square: int})");
        // Custom map name
        crosscheck_compare_only("(define-map a {x: int} {square: int})");
        // Custom map name duplicate
        crosscheck_compare_only(
            "(define-map a {x: int} {square: int}) (define-map a {x: int} {square: int})",
        );
    }
}
