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
    work_space: u32, // constant upper bound
) -> Result<(), GeneratorError> {
    let offset_res = generator.literal_memory_end;

    generator.literal_memory_end += mem_size as u32; // 5 u32

    let hash_type = match arg_types[0] {
        TypeSignature::IntType | TypeSignature::UIntType => {
            generator.ensure_work_space(work_space);
            "int"
        }
        TypeSignature::SequenceType(SequenceSubtype::BufferType(len)) => {
            // Input buff is also copied
            generator.ensure_work_space(u32::from(len) + work_space);
            "buf"
        }
        _ => {
            return Err(GeneratorError::NotImplemented);
        }
    };
    let hash_func = generator
        .module
        .funcs
        .by_name(&format!("stdlib.{name}-{hash_type}"))
        .ok_or_else(|| {
            GeneratorError::InternalError(format!("function not found: {name}-{hash_type}"))
        })?;

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

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // work_space values from sha256, see `Sha256::visit`
        traverse_hash("hash160", 160, generator, builder, arg_types, 64 + 8 + 289)
    }
}

#[derive(Debug)]
pub struct Sha256;

impl SimpleWord for Sha256 {
    fn name(&self) -> ClarityName {
        "sha256".into()
    }

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // work_space values from `standard.wat::$extend-data`: 64 for padding, 8 for padded size and 289 for the data shift
        traverse_hash("sha256", 256, generator, builder, arg_types, 64 + 8 + 289)
    }
}

#[derive(Debug)]
pub struct Keccak256;

impl SimpleWord for Keccak256 {
    fn name(&self) -> ClarityName {
        "keccak256".into()
    }

    fn visit(
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
                generator.write_to_memory(builder, buffer_local, 0, ty)?;

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
                .ok_or_else(|| {
                    GeneratorError::InternalError("stdlib.keccak256 not found".to_owned())
                })?,
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

    fn visit(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        // work_space values from `standard.wat::$pad-sha512-data`: 128 for padding, 16 for padded size and 705 for the data shift
        traverse_hash("sha512", 512, generator, builder, arg_types, 128 + 16 + 705)
    }
}

#[derive(Debug)]
pub struct Sha512_256;

impl SimpleWord for Sha512_256 {
    fn name(&self) -> ClarityName {
        "sha512/256".into()
    }

    fn visit(
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
                generator.write_to_memory(builder, buffer_local, 0, ty)?;

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

        // Call the host interface function, `sha512_256`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.sha512_256")
                .ok_or_else(|| {
                    GeneratorError::InternalError("stdlib.sha512_256 not found".to_owned())
                })?,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::crosscheck;

    #[test]
    fn test_keccak256() {
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "f490de2920c8a35fabeb13208852aa28c76f9be9b03a4dd2b3c075f7a26923b4",
            &mut expected,
        )
        .unwrap();
        crosscheck(
            "(keccak256 0)",
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }

    #[test]
    fn test_sha512() {
        let mut expected = [0u8; 64];
        hex::decode_to_slice(
            "6fcee9a7b7a7b821d241c03c82377928bc6882e7a08c78a4221199bfa220cdc55212273018ee613317c8293bb8d1ce08d1e017508e94e06ab85a734c99c7cc34",
            &mut expected,
        )
        .unwrap();
        crosscheck(
            "(sha512 1)",
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        );
    }

    #[test]
    fn test_sha512_overwrite() {
        let expected = [0u8; 64];
        crosscheck(
            "(sha512 1)
0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        );
    }

    #[test]
    fn test_sha512_256_overwrite() {
        let expected = [0u8; 32];
        crosscheck(
            "(sha512/256 1)
0x0000000000000000000000000000000000000000000000000000000000000000",
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        );
    }

    #[test]
    fn test_sha256_overwrite() {
        let expected = [0u8; 32];
        crosscheck(
            "(sha256 1)
0x0000000000000000000000000000000000000000000000000000000000000000",
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
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
        crosscheck(
            "(sha512/256 1)",
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }

    #[test]
    fn test_sha256_large_buff() {
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "c4145364a3ba46002fb14242872f795535bae6738b1e47ba21eb405cfdf820a5",
            &mut expected,
        )
        .unwrap();
        crosscheck(
            &format!("(sha256 0x{})", "aa".repeat(1048576)),
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }

    #[test]
    fn test_sha512_large_buff() {
        let mut expected = [0u8; 64];
        hex::decode_to_slice(
            "e3bbbc0cc37e452a5d2674240c77f7d5137b93fb9d4026b40a10a2ffeda543ff303df1220492cb9e8caba96c24aebb2d2ea359a38141b62d31d80996defdf874",
            &mut expected,
        )
        .unwrap();
        crosscheck(
            &format!("(sha512 0x{})", "aa".repeat(1048576)),
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }

    #[test]
    fn test_sha512256_large_buff() {
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "7d5b92a003008bb3ef9656e2212b27c47f325ecfba4ed78f1d7e83161bcaab4a",
            &mut expected,
        )
        .unwrap();
        crosscheck(
            &format!("(sha512/256 0x{})", "aa".repeat(1048576)),
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }

    #[test]
    fn test_hash160_large_buff() {
        let mut expected = [0u8; 20];
        hex::decode_to_slice("b7ec553926497b8cb2ae106bf75396359296830e", &mut expected).unwrap();
        crosscheck(
            &format!("(hash160 0x{})", "aa".repeat(1048576)),
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }

    #[test]
    fn test_keccak256_large_buff() {
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "b285806915c373a14ab20b503b1fe58a50544363263a1a17f50841ed08da85cb",
            &mut expected,
        )
        .unwrap();
        crosscheck(
            &format!("(keccak256 0x{})", "aa".repeat(1048576)),
            Ok(Some(Value::buff_from(expected.to_vec()).unwrap())),
        )
    }
}
