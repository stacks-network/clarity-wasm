use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct GetBlockInfo;

impl ComplexWord for GetBlockInfo {
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
        let (id_offset, id_length) = generator.add_string_literal(prop_name)?;
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the block number onto the stack
        generator.traverse_expr(builder, block)?;

        // Reserve space on the stack for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| {
                GeneratorError::TypeError("get-block-info? expression must be typed".to_owned())
            })?
            .clone();

        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &return_ty, true, true);

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `get_block_info`
        builder.call(generator.func_by_name("stdlib.get_block_info"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct GetBurnBlockInfo;

impl ComplexWord for GetBurnBlockInfo {
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
        let (id_offset, id_length) = generator.add_string_literal(prop_name)?;
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the block number onto the stack
        generator.traverse_expr(builder, block)?;

        // Reserve space on the stack for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| {
                GeneratorError::TypeError(
                    "get-burn-block-info? expression must be typed".to_owned(),
                )
            })?
            .clone();

        let (return_offset, return_size) =
            generator.create_call_stack_local(builder, &return_ty, true, true);

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `get_burn_block_info`
        builder.call(generator.func_by_name("stdlib.get_burn_block_info"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct AtBlock;

impl ComplexWord for AtBlock {
    fn name(&self) -> ClarityName {
        "at-block".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let block_hash = args.get_expr(0)?;
        let e = args.get_expr(1)?;

        // Traverse the block_hash, leaving it on the top of the stack
        generator.traverse_expr(builder, block_hash)?;

        // Call the host interface function, `enter_at_block`
        builder.call(generator.func_by_name("stdlib.enter_at_block"));

        // Traverse the inner expression
        generator.traverse_expr(builder, e)?;

        // Call the host interface function, `exit_at_block`
        builder.call(generator.func_by_name("stdlib.exit_at_block"));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::types::StacksEpochId;
    use clarity::vm::types::{OptionalData, PrincipalData, TupleData};
    use clarity::vm::Value;

    use crate::tools::{crosscheck, crosscheck_with_epoch, evaluate, TestEnvironment};

    //- Block Info

    #[test]
    fn get_block_info_non_existent() {
        crosscheck_with_epoch(
            "(get-block-info? time u9999999)",
            Ok(Some(Value::none())),
            StacksEpochId::Epoch25,
        );
    }

    #[test]
    fn get_block_info_burnchain_header_hash() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? burnchain-header-hash u0)")
            .expect("Failed to init contract.");
        assert_eq!(
            result,
            Some(Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    fn get_block_info_id_header_hash() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? id-header-hash u0)")
            .expect("Failed to init contract.");
        let mut expected = [0u8; 32];
        hex::decode_to_slice(
            "b5e076ab7609c7f8c763b5c571d07aea80b06b41452231b1437370f4964ed66e",
            &mut expected,
        )
        .unwrap();
        assert_eq!(
            result,
            Some(Value::some(Value::buff_from(expected.to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    fn get_block_info_header_hash() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? header-hash u0)")
            .expect("Failed to init contract.");
        assert_eq!(
            result,
            Some(Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap())
        );
    }

    #[test]
    fn get_block_info_miner_address() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? miner-address u0)")
            .expect("Failed to init contract.");
        assert_eq!(
            result,
            Some(
                Value::some(Value::Principal(
                    PrincipalData::parse("ST000000000000000000002AMW42H").unwrap()
                ))
                .unwrap()
            )
        )
    }

    #[test]
    fn get_block_info_time() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? time u0)")
            .expect("Failed to init contract.");
        let block_time_val = match result {
            Some(Value::Optional(OptionalData { data: Some(data) })) => *data,
            _ => panic!("expected value"),
        };
        let block_time = match block_time_val {
            Value::UInt(val) => val,
            _ => panic!("expected uint"),
        };
        let now = chrono::Utc::now().timestamp() as u128;

        // The block time should be close to the current time, so let's give it
        // a 10 second window, to be safe.
        assert!(block_time >= now - 10);
    }

    #[test]
    #[ignore = "block-reward is not simulated in the test framework"]
    fn get_block_info_block_reward() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? block-reward u0)")
            .expect("Failed to init contract.");
        assert_eq!(result, Some(Value::some(Value::UInt(0)).unwrap()));
    }

    #[test]
    fn get_block_info_miner_spend_total() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? miner-spend-total u0)")
            .expect("Failed to init contract.");
        assert_eq!(result, Some(Value::some(Value::UInt(0)).unwrap()));
    }

    #[test]
    fn get_block_info_miner_spend_winner() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-block-info? miner-spend-winner u0)")
            .expect("Failed to init contract.");
        assert_eq!(result, Some(Value::some(Value::UInt(0)).unwrap()));
    }

    //- Burn Block Info
    #[test]
    fn get_burn_block_info_non_existent() {
        crosscheck(
            "(get-burn-block-info? header-hash u9999999)",
            Ok(Some(
                Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap(),
            )),
        )
    }

    #[test]
    fn get_burn_block_info_header_hash() {
        crosscheck(
            "(get-burn-block-info? header-hash u0)",
            Ok(Some(
                Value::some(Value::buff_from([0; 32].to_vec()).unwrap()).unwrap(),
            )),
        )
    }

    #[test]
    fn get_burn_block_info_pox_addrs() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        let result = env
            .evaluate("(get-burn-block-info? pox-addrs u0)")
            .expect("Failed to init contract.");
        assert_eq!(
            result,
            Some(
                Value::some(
                    TupleData::from_data(vec![
                        (
                            "addrs".into(),
                            Value::cons_list_unsanitized(vec![TupleData::from_data(vec![
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

    //- At Block
    #[test]
    fn at_block() {
        crosscheck_with_epoch("(at-block 0x0000000000000000000000000000000000000000000000000000000000000000 block-height)",
            Ok(Some(Value::UInt(0xFFFFFFFF))),
            StacksEpochId::Epoch24,
        )
    }

    //- At Block
    #[test]
    fn at_block_with_stacks_block_height() {
        crosscheck_with_epoch("(at-block 0x0000000000000000000000000000000000000000000000000000000000000000 stacks-block-height)",
            Ok(Some(Value::UInt(0xFFFFFFFF))),
            StacksEpochId::Epoch30,
        )
    }

    #[test]
    fn at_block_var() {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(1);
        // Should error, since the data var is not yet defined in block 0
        let e = env
            .evaluate(
                r#"
(define-data-var data int 1)
(at-block (unwrap-panic (get-block-info? id-header-hash u0))
    (var-get data)
)
"#,
            )
            .unwrap_err();
        println!("{:?}", e);
    }

    #[test]
    fn test_block_height() {
        let snpt = "
(define-public (block)
  (ok block-height))

(define-public (burn-block)
  (ok burn-block-height))
";

        crosscheck_with_epoch(
            &format!("{snpt} (block)"),
            evaluate("(ok u0)"),
            StacksEpochId::Epoch24,
        );
        crosscheck_with_epoch(
            &format!("{snpt} (burn-block)"),
            evaluate("(ok u0)"),
            StacksEpochId::Epoch24,
        );
    }

    #[test]
    fn test_chain_id() {
        crosscheck(
            "
(define-public (get-chain-id)
  (ok chain-id))

(get-chain-id)
",
            evaluate("(ok u2147483648)"),
        );
    }
}
