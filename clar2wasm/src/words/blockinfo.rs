use clarity::vm::{ClarityName, SymbolicExpression};

use super::ComplexWord;
use crate::check_args;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use crate::wasm_utils::{check_argument_count, ArgumentCountCheck};

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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let prop_name = args.get_name(0)?;
        let block = args.get_expr(1)?;

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
        // Parse the property name at compile time
        match prop_name.as_str() {
            "time" => {
                builder.call(generator.func_by_name("stdlib.get_block_info_time_property"));
            }
            "header-hash" => {
                builder.call(generator.func_by_name("stdlib.get_block_info_header_hash_property"));
            }
            "burnchain-header-hash" => {
                builder.call(
                    generator.func_by_name("stdlib.get_block_info_burnchain_header_hash_property"),
                );
            }
            "id-header-hash" => {
                builder.call(
                    generator.func_by_name("stdlib.get_block_info_identity_header_hash_property"),
                );
            }
            "miner-address" => {
                builder
                    .call(generator.func_by_name("stdlib.get_block_info_miner_address_property"));
            }
            "block-reward" => {
                builder.call(generator.func_by_name("stdlib.get_block_info_block_reward_property"));
            }
            "miner-spend-total" => {
                builder.call(
                    generator.func_by_name("stdlib.get_block_info_miner_spend_total_property"),
                );
            }
            "miner-spend-winner" => {
                builder.call(
                    generator.func_by_name("stdlib.get_block_info_miner_spend_winner_property"),
                );
            }
            _ => {
                return Err(GeneratorError::InternalError(format!(
                    "{self:?} does not have a property of type {}",
                    prop_name
                )))
            }
        };

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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

        let prop_name = args.get_name(0)?;
        let block = args.get_expr(1)?;

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

        match prop_name.as_str() {
            "header-hash" => {
                builder.call(
                    generator.func_by_name("stdlib.get_burn_block_info_header_hash_property"),
                );
            }
            "pox-addrs" => {
                builder
                    .call(generator.func_by_name("stdlib.get_burn_block_info_pox_addrs_property"));
            }
            _ => {
                return Err(GeneratorError::InternalError(format!(
                    "{self:?} does not have a property of type {}",
                    prop_name
                )))
            }
        };

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
        check_args!(generator, builder, 2, args.len(), ArgumentCountCheck::Exact);

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
    use clarity::vm::errors::{CheckErrors, Error};
    use clarity::vm::types::{OptionalData, PrincipalData, TupleData};
    use clarity::vm::Value;

    use crate::tools::{evaluate, TestEnvironment};

    //
    // Module with tests that should only be executed
    // when running Clarity::V1 or Clarity::V2.
    //
    #[cfg(any(feature = "test-clarity-v1", feature = "test-clarity-v2"))]
    #[cfg(test)]
    mod clarity_v1_v2 {
        use clarity::types::StacksEpochId;

        use super::*;
        use crate::tools::crosscheck_with_epoch;

        #[test]
        fn get_block_info_non_existent() {
            crosscheck_with_epoch(
                "(get-block-info? time u9999999)",
                Ok(Some(Value::none())),
                StacksEpochId::Epoch25,
            );
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
        fn at_block() {
            crosscheck_with_epoch(
                "(at-block 0x0000000000000000000000000000000000000000000000000000000000000000 block-height)",
                Ok(Some(Value::UInt(0xFFFFFFFF))),
                StacksEpochId::Epoch24,
            )
        }

        #[test]
        fn get_block_info_less_than_two_args() {
            let mut env = TestEnvironment::default();
            env.advance_chain_tip(1);
            let result = env.evaluate("(get-block-info? id-header-hash)");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("expecting >= 2 arguments, got 1"));
        }

        #[test]
        fn get_block_info_more_than_two_args() {
            // TODO: see issue #488
            // The inconsistency in function arguments should have been caught by the typechecker.
            // The runtime error below is being used as a workaround for a typechecker issue
            // where certain errors are not properly handled.
            // This test should be re-worked once the typechecker is fixed
            // and can correctly detect all argument inconsistencies.
            let snippet = "(get-block-info? burnchain-header-hash u0 miner-address)";
            let expected = Err(Error::Unchecked(CheckErrors::IncorrectArgumentCount(2, 3)));
            crosscheck_with_epoch(snippet, expected, StacksEpochId::Epoch24);
        }
    }

    //
    // Module with tests that should only be executed
    // when running Clarity::V3.
    //
    #[cfg(feature = "test-clarity-v3")]
    mod clarity_v3 {
        use clarity::types::StacksEpochId;

        use super::*;
        use crate::tools::crosscheck_with_epoch;

        //- At Block
        #[test]
        fn at_block_with_stacks_block_height() {
            crosscheck_with_epoch("(at-block 0x0000000000000000000000000000000000000000000000000000000000000000 stacks-block-height)",
                Ok(Some(Value::UInt(0xFFFFFFFF))),
                StacksEpochId::Epoch30,
            )
        }

        #[test]
        fn get_stacks_block_info_less_than_two_args() {
            let result = evaluate("(get-stacks-block-info? id-header-hash)");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("expecting 2 arguments, got 1"));
        }

        #[test]
        fn get_stacks_block_info_more_than_two_args() {
            let result = evaluate("(get-stacks-block-info? id-header-hash u0 u0)");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("expecting 2 arguments, got 3"));
        }
    }

    //- Block Info

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

    #[test]
    fn get_burn_block_info_less_than_two_args() {
        let result = evaluate("(get-burn-block-info? id-header-hash)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn get_burn_block_info_more_than_two_args() {
        let result = evaluate("(get-burn-block-info? id-header-hash u0 u0)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
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

    #[test]
    fn at_block_less_than_two_args() {
        let result = evaluate(
            "(at-block 0xb5e076ab7609c7f8c763b5c571d07aea80b06b41452231b1437370f4964ed66e)",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 1"));
    }

    #[test]
    fn at_block_more_than_two_args() {
        let result = evaluate(
            "(at-block 0xb5e076ab7609c7f8c763b5c571d07aea80b06b41452231b1437370f4964ed66e u0 u0)",
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 2 arguments, got 3"));
    }

    #[test]
    fn at_block_var() {
        let e = evaluate(
                "
(define-data-var data int 1)
(at-block 0xb5e076ab7609c7f8c763b5c571d07aea80b06b41452231b1437370f4964ed66e (var-get data)) ;; block 0
",
            )
            .unwrap_err();
        assert_eq!(
            e,
            Error::Unchecked(CheckErrors::NoSuchDataVariable("data".into()))
        );
    }

    //
    // Module with tests that should only be executed
    // when running Clarity::V2 or Clarity::v3.
    //
    #[cfg(not(feature = "test-clarity-v1"))]
    #[cfg(test)]
    mod clarity_v2_v3 {
        use super::*;
        use crate::tools::crosscheck;

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
}
