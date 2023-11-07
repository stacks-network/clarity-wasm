use crate::wasm_generator::ArgumentsExt;
use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

#[derive(Debug)]
pub struct GetBlockInfo;

impl Word for GetBlockInfo {
    fn name(&self) -> ClarityName {
        "get-block-info?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let prop_name = args.get_name(0)?;
        let block = args.get_expr(1)?;

        // Push the property name onto the stack
        let (id_offset, id_length) = generator.add_string_literal(prop_name);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the block number onto the stack
        generator.traverse_expr(builder, block)?;

        // Reserve space on the stack for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .expect("get-block-info? expression must be typed")
            .clone();

        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &return_ty, true, true);

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `get_block_info`
        builder.call(generator.func_by_name("get_block_info"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty);

        Ok(())
    }
}

#[derive(Debug)]
pub struct GetBurnBlockInfo;

impl Word for GetBurnBlockInfo {
    fn name(&self) -> ClarityName {
        "get-burn-block-info?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let prop_name = args.get_name(0)?;
        let block = args.get_expr(1)?;

        // Push the property name onto the stack
        let (id_offset, id_length) = generator.add_string_literal(prop_name);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the block number onto the stack
        generator.traverse_expr(builder, block)?;

        // Reserve space on the stack for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .expect("get-burn-block-info? expression must be typed")
            .clone();

        println!("return_ty: {:?}", return_ty);

        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &return_ty, true, true);

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `get_burn_block_info`
        builder.call(generator.func_by_name("get_burn_block_info"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::{types::{TupleData, StandardPrincipalData}, Value};

    use crate::tools::evaluate;

    // These tests are disabled because they require a block to be present in the
    // chain, which is not the case when running the tests. Once the test framework
    // supports this, these tests can be re-enabled.

    //- Block Info

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_non_existent() {
        assert_eq!(
            evaluate("(get-block-info? time u9999999)"),
            Some(Value::none())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_burnchain_header_hash() {
        assert_eq!(
            evaluate("(get-block-info? burnchain-header-hash u0)"),
            Some(Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_id_header_hash() {
        assert_eq!(
            evaluate("(get-block-info? id-header-hash u0)"),
            Some(Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_header_hash() {
        assert_eq!(
            evaluate("(get-block-info? header-hash u0)"),
            Some(Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_miner_address() {
        assert_eq!(
            evaluate("(get-block-info? miner-address u0)"),
            Some(Value::some(StandardPrincipalData::transient().into()).unwrap())
        )
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_time() {
        assert_eq!(
            evaluate("(get-block-info? time u0)"),
            Some(Value::some(Value::UInt(0)).unwrap())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_block_reward() {
        assert_eq!(
            evaluate("(get-block-info? block-reward u0)"),
            Some(Value::some(Value::UInt(0)).unwrap())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_miner_spend_total() {
        assert_eq!(
            evaluate("(get-block-info? miner-spend-total u0)"),
            Some(Value::some(Value::UInt(0)).unwrap())
        );
    }

    #[test]
    #[ignore = "block info is not yet available in the test framework"]
    fn get_block_info_miner_spend_winner() {
        assert_eq!(
            evaluate("(get-block-info? miner-spend-winner u0)"),
            Some(Value::some(Value::UInt(0)).unwrap())
        );
    }

    //- Burn Block Info

    #[test]
    #[ignore = "burn block info is not yet available in the test framework"]
    fn get_burn_block_info_non_existent() {
        assert_eq!(
            evaluate("(get-burn-block-info? time u9999999)"),
            Some(Value::none())
        );
    }

    #[test]
    #[ignore = "burn block info is not yet available in the test framework"]
    fn get_burn_block_info_header_hash() {
        assert_eq!(
            evaluate("(get-burn-block-info? header-hash u0)"),
            Some(Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    #[ignore = "burn block info is not yet available in the test framework"]
    fn get_burn_block_info_pox_addrs() {
        assert_eq!(
            evaluate("(get-burn-block-info? pox-addrs u0)"),
            Some(
                Value::some(
                    TupleData::from_data(vec![
                        (
                            "addrs".into(),
                            Value::list_from(vec![TupleData::from_data(vec![
                                (
                                    "hashbytes".into(),
                                    Value::buff_from([0; 32].to_vec()).unwrap()
                                ),
                                ("version".into(), Value::buff_from_byte(0))
                            ])
                            .unwrap()
                            .into()])
                            .unwrap()
                        ),
                        ("payout".into(), Value::UInt(0))
                    ])
                    .unwrap()
                    .into()
                )
                .unwrap()
            )
        );
    }
}
