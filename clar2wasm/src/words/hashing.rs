use clarity::vm::types::{SequenceSubtype, TypeSignature, BUFF_32};
use clarity::vm::ClarityName;

use super::SimpleWord;
use crate::wasm_generator::{GeneratorError, WasmGenerator};

pub fn traverse_hash(
    name: &'static str,
    mem_size: usize,
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    arg_types: &[TypeSignature],
) -> Result<(), GeneratorError> {
    let offset_res = generator.literal_memory_end;

    generator.literal_memory_end += mem_size as u32; // 5 u32

    let hash_type = match arg_types[0] {
        TypeSignature::IntType | TypeSignature::UIntType => "int",
        TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => "buf",
        _ => {
            return Err(GeneratorError::NotImplemented);
        }
    };
    let hash_func = generator
        .module
        .funcs
        .by_name(&format!("stdlib.{name}-{hash_type}"))
        .unwrap_or_else(|| panic!("function not found: {name}-{hash_type}"));

    builder
        .i32_const(offset_res as i32) // result offset
        .call(hash_func);

    Ok(())
}

#[derive(Debug)]
pub struct Hash160;

impl SimpleWord for Hash160 {
    fn name(&self) -> ClarityName {
        "hash160".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        traverse_hash(
            "hash160",
            core::mem::size_of::<u32>() * 5,
            generator,
            builder,
            arg_types,
        )
    }
}

#[derive(Debug)]
pub struct Sha256;

impl SimpleWord for Sha256 {
    fn name(&self) -> ClarityName {
        "sha256".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        traverse_hash(
            "sha256",
            core::mem::size_of::<u32>() * 8,
            generator,
            builder,
            arg_types,
        )
    }
}

#[derive(Debug)]
pub struct Keccak256;

impl SimpleWord for Keccak256 {
    fn name(&self) -> ClarityName {
        "keccak256".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let ty = &arg_types[0];
        match ty {
            TypeSignature::IntType | TypeSignature::UIntType => {
                // Convert integers to buffers by storing them to memory
                let (buffer_local, size) =
                    generator.create_call_stack_local(builder, ty, false, true);
                generator.write_to_memory(builder, buffer_local, 0, ty);

                // The load the offset and length onto the stack
                builder.local_get(buffer_local).i32_const(size);
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => {}
            _ => {
                return Err(GeneratorError::TypeError(
                    "invalid type for keccak256".to_string(),
                ))
            }
        }

        // Reserve stack space for the host-function to write the result
        let ret_ty = BUFF_32.clone();
        let (result_local, result_size) =
            generator.create_call_stack_local(builder, &ret_ty, false, true);
        builder.local_get(result_local).i32_const(result_size);

        // Call the host interface function, `keccak256`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.keccak256")
                .expect("function not found"),
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct Sha512;

impl SimpleWord for Sha512 {
    fn name(&self) -> ClarityName {
        "sha512".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        traverse_hash(
            "sha512",
            core::mem::size_of::<u32>() * 8,
            generator,
            builder,
            arg_types,
        )
    }
}

#[derive(Debug)]
pub struct Sha512_256;

impl SimpleWord for Sha512_256 {
    fn name(&self) -> ClarityName {
        "sha512/256".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let ty = &arg_types[0];
        match ty {
            TypeSignature::IntType | TypeSignature::UIntType => {
                // Convert integers to buffers by storing them to memory
                let (buffer_local, size) =
                    generator.create_call_stack_local(builder, ty, false, true);
                generator.write_to_memory(builder, buffer_local, 0, ty);

                // The load the offset and length onto the stack
                builder.local_get(buffer_local).i32_const(size);
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => {}
            _ => {
                return Err(GeneratorError::TypeError(
                    "invalid type for sha512/256".to_string(),
                ))
            }
        }

        // Reserve stack space for the host-function to write the result
        let ret_ty = BUFF_32.clone();
        let (result_local, result_size) =
            generator.create_call_stack_local(builder, &ret_ty, false, true);
        builder.local_get(result_local).i32_const(result_size);

        // Call the host interface function, `sha512`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.sha512_256")
                .expect("function not found"),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::evaluate;

    #[test]
    fn test_keccak256() {
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "f490de2920c8a35fabeb13208852aa28c76f9be9b03a4dd2b3c075f7a26923b4",
            &mut expected,
        )
        .unwrap();
        assert_eq!(
            evaluate("(keccak256 0)"),
            Some(Value::buff_from(expected.to_vec()).unwrap())
        );
    }

    #[test]
    fn test_sha512() {
        let mut expected = [0u8; 64];
        hex::decode_to_slice(
            "6fcee9a7b7a7b821d241c03c82377928bc6882e7a08c78a4221199bfa220cdc55212273018ee613317c8293bb8d1ce08d1e017508e94e06ab85a734c99c7cc34",
            &mut expected,
        )
        .unwrap();
        assert_eq!(
            evaluate("(sha512 1)"),
            Some(Value::buff_from(expected.to_vec()).unwrap())
        );
    }

    #[test]
    fn test_sha512_256() {
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "515a7e92e7c60522db968d81ff70b80818fc17aeabbec36baf0dda2812e94a86",
            &mut expected,
        )
        .unwrap();
        assert_eq!(
            evaluate("(sha512/256 1)"),
            Some(Value::buff_from(expected.to_vec()).unwrap())
        );
    }
}
