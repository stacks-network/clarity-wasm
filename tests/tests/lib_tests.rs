use clar2wasm::compile;
use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clarity::{
    consts::CHAIN_ID_TESTNET,
    types::StacksEpochId,
    vm::{
        callables::DefineType,
        clarity_wasm::{call_function, initialize_contract},
        contexts::{CallStack, GlobalContext},
        costs::LimitedCostTracker,
        database::{ClarityDatabase, MemoryBackingStore},
        errors::{Error, WasmError},
        types::{
            PrincipalData, QualifiedContractIdentifier, ResponseData, StandardPrincipalData,
            TypeSignature,
        },
        ClarityVersion, ContractContext, ContractName, Value,
    },
};

/// This macro provides a convenient way to test contract initialization.
/// In order, it takes as parameters:
/// - the name of the test to create,
/// - the name of the contract containing the function,
/// - a closure with type
///  `|global_context: &mut GlobalContext, contract_context: &ContractContext|`
///   and that contains all the assertions we want to test.
macro_rules! test_contract_init {
    ($func: ident, $contract_name: literal, $context_test: expr) => {
        #[test]
        fn $func() {
            let contract_id = QualifiedContractIdentifier::new(
                StandardPrincipalData::transient(),
                ContractName::from($contract_name),
            );
            let mut datastore = Datastore::new();
            let constants = StacksConstants::default();
            let burn_datastore = BurnDatastore::new(constants);
            let mut clarity_store = MemoryBackingStore::new();
            let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
            conn.begin();
            conn.set_clarity_epoch_version(StacksEpochId::latest());
            conn.commit();
            let cost_tracker = LimitedCostTracker::new_free();
            let mut contract_context =
                ContractContext::new(contract_id.clone(), ClarityVersion::latest());

            let contract_str =
                std::fs::read_to_string(format!("contracts/{}.clar", $contract_name)).unwrap();
            let mut compile_result = compile(
                contract_str.as_str(),
                &contract_id,
                cost_tracker,
                ClarityVersion::latest(),
                StacksEpochId::latest(),
                &mut clarity_store,
            )
            .expect("Failed to compile contract.");

            contract_context.set_wasm_module(compile_result.module.emit_wasm());

            let mut global_context = GlobalContext::new(
                false,
                CHAIN_ID_TESTNET,
                conn,
                compile_result.contract_analysis.cost_track.take().unwrap(),
                StacksEpochId::latest(),
            );
            global_context.begin();

            {
                initialize_contract(
                    &mut global_context,
                    &mut contract_context,
                    None,
                    &compile_result.contract_analysis,
                )
                .expect("Failed to initialize contract.");

                // Give an account an initial balance
                let recipient = PrincipalData::Standard(StandardPrincipalData::transient());
                let amount = 1_000_000_000;
                let mut snapshot = global_context.database.get_stx_balance_snapshot(&recipient);
                snapshot.credit(amount);
                snapshot.save();
                global_context
                    .database
                    .increment_ustx_liquid_supply(amount)
                    .unwrap();

                // https://github.com/rust-lang/rust-clippy/issues/1553
                #[allow(clippy::redundant_closure_call)]
                $context_test(&mut global_context, &contract_context);
            }

            global_context.commit().unwrap();
        }
    };

    ($func: ident, $contract_name: literal, $contract_func: literal, $test: expr) => {
        test_contract_init!($func, $contract_name, $contract_func, &[], $test);
    };
}

/// This macro provides a convenient way to test functions inside contracts.
/// In order, it takes as parameters:
/// - the name of the test to create,
/// - the name of the contract containing the function,
/// - the name of the function to test,
/// - an optional list of parameters,
/// - a closure with type `|result: Result<Value, Error>|`, and
///   that contains all the assertions we want to test.
macro_rules! test_contract_call {
    ($func: ident, $contract_name: literal, $contract_func: literal, $params: expr, $test: expr) => {
        test_contract_init!(
            $func,
            $contract_name,
            |global_context: &mut GlobalContext, contract_context: &ContractContext| {
                // Initialize a call stack
                let mut call_stack = CallStack::new();

                let result = call_function(
                    $contract_func,
                    $params,
                    global_context,
                    contract_context,
                    &mut call_stack,
                    Some(StandardPrincipalData::transient().into()),
                    Some(StandardPrincipalData::transient().into()),
                    None,
                );

                // https://github.com/rust-lang/rust-clippy/issues/1553
                #[allow(clippy::redundant_closure_call)]
                $test(result);
            }
        );
    };

    ($func: ident, $contract_name: literal, $contract_func: literal, $test: expr) => {
        test_contract_call!($func, $contract_name, $contract_func, &[], $test);
    };
}

/// This macro provides a convenient way to test functions inside contracts.
/// In order, it takes as parameters:
/// - the name of the test to create,
/// - the name of the contract containing the function,
/// - the name of the function to test,
/// - an optional list of parameters,
/// - a closure with type `|response: ResponseData|`, and
///   that contains all the assertions we want to test.
macro_rules! test_contract_call_response {
    ($func: ident, $contract_name: literal, $contract_func: literal, $params: expr, $test: expr) => {
        test_contract_call!(
            $func,
            $contract_name,
            $contract_func,
            $params,
            |result: Result<Value, Error>| {
                let result = result.expect("Function call failed.");

                if let Value::Response(response_data) = result {
                    // https://github.com/rust-lang/rust-clippy/issues/1553
                    #[allow(clippy::redundant_closure_call)]
                    $test(response_data);
                } else {
                    panic!("Unexpected result received from WASM function call.");
                }
            }
        );
    };

    ($func: ident, $contract_name: literal, $contract_func: literal, $test: expr) => {
        test_contract_call_response!($func, $contract_name, $contract_func, &[], $test);
    };
}

test_contract_call_response!(test_add, "add", "simple", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::Int(3));
});

test_contract_init!(
    test_define_private,
    "call-private-with-args",
    |_global_context: &mut GlobalContext, contract_context: &ContractContext| {
        let public_function = contract_context.lookup_function("simple").unwrap();
        assert_eq!(public_function.define_type, DefineType::Private);
        assert_eq!(
            public_function.get_arg_types(),
            &[TypeSignature::IntType, TypeSignature::IntType]
        );
        assert_eq!(
            public_function.get_return_type(),
            &Some(TypeSignature::IntType)
        );
    }
);

test_contract_call_response!(
    test_call_private_with_args_nested,
    "call-private-with-args",
    "call-it",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract_call_response!(
    test_call_public,
    "call-public",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract_call_response!(
    test_call_public_nested,
    "call-public",
    "call-it",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract_call_response!(
    test_call_public_with_args,
    "call-public-with-args",
    "simple",
    &[Value::Int(20), Value::Int(22)],
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract_call_response!(
    test_call_public_with_args_nested,
    "call-public-with-args",
    "call-it",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract_init!(
    test_define_public,
    "define-public-ok",
    |_global_context: &mut GlobalContext, contract_context: &ContractContext| {
        let public_function = contract_context.lookup_function("simple").unwrap();
        assert_eq!(public_function.define_type, DefineType::Public);
        assert!(public_function.get_arg_types().is_empty());
        assert_eq!(
            public_function.get_return_type(),
            &Some(TypeSignature::ResponseType(Box::new((
                TypeSignature::IntType,
                TypeSignature::NoType
            ))))
        );
    }
);

test_contract_call_response!(
    test_define_public_err,
    "define-public-err",
    "simple",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract_call_response!(
    test_define_public_ok,
    "define-public-ok",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract_init!(
    test_define_data_var,
    "var-get",
    |_global_context: &mut GlobalContext, contract_context: &ContractContext| {
        let metadata = contract_context.meta_data_var.get("something").unwrap();
        assert_eq!(metadata.value_type, TypeSignature::IntType);
    }
);

test_contract_call_response!(
    test_var_get,
    "var-get",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(123));
    }
);

test_contract_call_response!(
    test_var_set,
    "var-set",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(0x123_0000_0000_0000_0456));
    }
);

test_contract_call_response!(test_fold, "fold", "fold-sub", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::Int(2));
});

test_contract_call_response!(test_begin, "begin", "simple", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::Int(7));
});

test_contract_call_response!(
    test_less_than,
    "cmp-arith",
    "less-uint",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_greater_or_equal,
    "cmp-arith",
    "greater-or-equal-int",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_bitwise_and,
    "bit-and",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract_call_response!(
    test_bitwise_not,
    "bit-not",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(-4));
    }
);

test_contract_call_response!(
    test_bitwise_or,
    "bit-or",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract_call_response!(
    test_bitwise_shift_left,
    "bit-shift-left",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(6));
    }
);

test_contract_call_response!(
    test_bitwise_shift_right,
    "bit-shift-right",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract_call_response!(
    test_bitwise_xor,
    "bit-xor",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(1));
    }
);

test_contract_call_response!(
    test_fold_bench,
    "fold-bench",
    "fold-add-square",
    &[
        Value::list_from((1..=8192).map(Value::Int).collect())
            .expect("failed to construct list argument"),
        Value::Int(1)
    ],
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(183285493761));
    }
);

test_contract_call_response!(
    test_ret_true,
    "bool",
    "ret-true",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_ret_false,
    "bool",
    "ret-false",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(false));
    }
);

test_contract_call_response!(
    test_block_height,
    "block-heights",
    "block",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(0));
    }
);

test_contract_call_response!(
    test_burn_block_height,
    "block-heights",
    "burn-block",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(0));
    }
);

test_contract_call_response!(
    test_chain_id,
    "chain-id",
    "get-chain-id",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(2147483648));
    }
);

test_contract_call_response!(
    test_tx_sender,
    "builtins-principals",
    "get-tx-sender",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::Principal(PrincipalData::Standard(StandardPrincipalData::transient()))
        );
    }
);

test_contract_call_response!(
    test_contract_caller,
    "builtins-principals",
    "get-contract-caller",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::Principal(PrincipalData::Standard(StandardPrincipalData::transient()))
        );
    }
);

test_contract_call_response!(
    test_tx_sponsor,
    "builtins-principals",
    "get-tx-sponsor",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::none(),);
    }
);

test_contract_call_response!(
    test_is_in_mainnet,
    "network",
    "mainnet",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(false));
    }
);

test_contract_call_response!(
    test_is_in_regtest,
    "network",
    "regtest",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(false));
    }
);

test_contract_call_response!(test_none, "none", "ret-none", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::none());
});

test_contract_call_response!(
    test_as_contract_sender,
    "as-contract",
    "check-sender",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::Principal(PrincipalData::Contract(QualifiedContractIdentifier {
                issuer: StandardPrincipalData::transient(),
                name: "as-contract".into()
            }))
        );
    }
);

test_contract_call_response!(
    test_as_contract_caller,
    "as-contract",
    "check-caller",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::Principal(PrincipalData::Contract(QualifiedContractIdentifier {
                issuer: StandardPrincipalData::transient(),
                name: "as-contract".into()
            }))
        );
    }
);

test_contract_call_response!(
    test_stx_get_balance,
    "stx-funcs",
    "test-stx-get-balance",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(0));
    }
);

test_contract_call_response!(
    test_stx_account,
    "stx-funcs",
    "test-stx-account",
    |response: ResponseData| {
        assert!(response.committed);
        match *response.data {
            Value::Tuple(tuple_data) => {
                assert_eq!(tuple_data.data_map.len(), 3);
                assert_eq!(tuple_data.data_map.get("locked").unwrap(), &Value::UInt(0));
                assert_eq!(
                    tuple_data.data_map.get("unlocked").unwrap(),
                    &Value::UInt(0)
                );
                assert_eq!(
                    tuple_data.data_map.get("unlock-height").unwrap(),
                    &Value::UInt(0)
                );
            }
            _ => panic!("Unexpected result received from WASM function call."),
        }
    }
);

test_contract_call_response!(
    test_stx_burn_ok,
    "stx-funcs",
    "test-stx-burn-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_stx_burn_err1,
    "stx-funcs",
    "test-stx-burn-err1",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_stx_burn_err3,
    "stx-funcs",
    "test-stx-burn-err3",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(3));
    }
);

test_contract_call_response!(
    test_stx_burn_err4,
    "stx-funcs",
    "test-stx-burn-err4",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(4));
    }
);

test_contract_call_response!(
    test_stx_transfer_ok,
    "stx-funcs",
    "test-stx-transfer-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_stx_transfer_memo_ok,
    "stx-funcs",
    "test-stx-transfer-memo-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_stx_transfer_err1,
    "stx-funcs",
    "test-stx-transfer-err1",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_stx_transfer_err2,
    "stx-funcs",
    "test-stx-transfer-err2",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(2));
    }
);

test_contract_call_response!(
    test_stx_transfer_err3,
    "stx-funcs",
    "test-stx-transfer-err3",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(3));
    }
);

test_contract_call_response!(
    test_stx_transfer_err4,
    "stx-funcs",
    "test-stx-transfer-err4",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(4));
    }
);

test_contract_call_response!(
    test_pow_int,
    "power",
    "with-int",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(230539333248));
    }
);

test_contract_call_response!(
    test_pow_uint,
    "power",
    "with-uint",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::UInt(311973482284542371301330321821976049)
        );
    }
);

test_contract_init!(
    test_define_ft,
    "define-tokens",
    |_global_context: &mut GlobalContext, contract_context: &ContractContext| {
        let ft_metadata = contract_context
            .meta_ft
            .get("foo")
            .expect("FT 'foo' not found");
        assert_eq!(ft_metadata.total_supply, None);

        let ft_metadata = contract_context
            .meta_ft
            .get("bar")
            .expect("FT 'bar' not found");
        assert_eq!(ft_metadata.total_supply, Some(1000000u128));
    }
);

test_contract_init!(
    test_define_nft,
    "define-tokens",
    |_global_context: &mut GlobalContext, contract_context: &ContractContext| {
        let nft_metadata = contract_context
            .meta_nft
            .get("baz")
            .expect("NFT 'baz' not found");
        assert_eq!(nft_metadata.key_type, TypeSignature::UIntType);
    }
);

test_contract_call_response!(
    test_int_constant,
    "constant",
    "get-int-constant",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(1));
    }
);

test_contract_call_response!(
    test_large_uint_constant,
    "constant",
    "get-large-uint-constant",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::UInt(338770000845734292516042252062085074415)
        );
    }
);

test_contract_call_response!(
    test_string_constant,
    "constant",
    "get-string-constant",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::string_ascii_from_bytes(b"hello world".to_vec()).unwrap()
        );
    }
);

test_contract_call_response!(
    test_bytes_constant,
    "constant",
    "get-bytes-constant",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::buff_from(vec![0x12, 0x34, 0x56, 0x78]).unwrap()
        );
    }
);

test_contract_init!(
    test_define_map,
    "define-map",
    |_global_context: &mut GlobalContext, contract_context: &ContractContext| {
        let map_metadata = contract_context
            .meta_data_map
            .get("my-map")
            .expect("Map 'my-map' not found");
        assert_eq!(map_metadata.key_type, TypeSignature::PrincipalType);
        assert_eq!(map_metadata.value_type, TypeSignature::UIntType);
    }
);

test_contract_call_response!(
    test_ft_get_supply_0,
    "tokens",
    "foo-get-supply-0",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(0));
    }
);

test_contract_call_response!(
    test_ft_mint,
    "tokens",
    "foo-mint",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_ft_mint_0,
    "tokens",
    "foo-mint-0",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call!(
    test_ft_mint_too_many,
    "tokens",
    "bar-mint-too-many",
    |result: Result<Value, Error>| {
        // Expecting a RuntimeErrorType::SupplyOverflow(1000001, 1000000)
        assert!(matches!(result, Err(Error::Wasm(WasmError::Runtime(_)))));
    }
);

test_contract_call!(
    test_ft_mint_too_many_2,
    "tokens",
    "bar-mint-too-many-2",
    |result: Result<Value, Error>| {
        // Expecting a RuntimeErrorType::SupplyOverflow(11111110, 1000000)
        assert!(matches!(result, Err(Error::Wasm(WasmError::Runtime(_)))));
    }
);

test_contract_call_response!(
    test_ft_balance_0,
    "tokens",
    "ft-balance-0",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(0));
    }
);

test_contract_call_response!(
    test_ft_balance_10,
    "tokens",
    "ft-balance-10",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(10));
    }
);

test_contract_call_response!(
    test_ft_burn_unowned,
    "tokens",
    "ft-burn-unowned",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_ft_burn_0,
    "tokens",
    "ft-burn-0",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_ft_burn_ok,
    "tokens",
    "ft-burn-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_ft_burn_too_many,
    "tokens",
    "ft-burn-too-many",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_ft_burn_other,
    "tokens",
    "ft-burn-other",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_ft_transfer_unowned,
    "tokens",
    "ft-transfer-unowned",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_ft_transfer_0,
    "tokens",
    "ft-transfer-0",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(3));
    }
);

test_contract_call_response!(
    test_ft_transfer_ok,
    "tokens",
    "ft-transfer-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_ft_transfer_too_many,
    "tokens",
    "ft-transfer-too-many",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_ft_transfer_other,
    "tokens",
    "ft-transfer-other",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_ft_transfer_self,
    "tokens",
    "ft-transfer-self",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(2));
    }
);

test_contract_call_response!(
    test_nft_mint,
    "tokens",
    "nft-mint",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_nft_mint_other,
    "tokens",
    "nft-mint-other",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_nft_mint_duplicate,
    "tokens",
    "nft-mint-duplicate",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_nft_get_owner,
    "tokens",
    "nft-get-owner",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(
            *response.data,
            Value::some(Value::Principal(
                PrincipalData::parse("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap()
            ))
            .unwrap()
        );
    }
);

test_contract_call_response!(
    test_nft_get_owner_unowned,
    "tokens",
    "nft-get-owner-unowned",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::none(),);
    }
);

test_contract_call_response!(
    test_nft_burn,
    "tokens",
    "nft-burn",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_nft_burn_other,
    "tokens",
    "nft-burn-other",
    |response: ResponseData| {
        println!("{:?}", response);
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_nft_burn_wrong,
    "tokens",
    "nft-burn-wrong",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(3));
    }
);

test_contract_call_response!(
    test_nft_transfer_does_not_exist,
    "tokens",
    "nft-transfer-does-not-exist",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(3));
    }
);

test_contract_call_response!(
    test_nft_transfer_ok,
    "tokens",
    "nft-transfer-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_nft_transfer_wrong,
    "tokens",
    "nft-transfer-wrong",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(3));
    }
);

test_contract_call_response!(
    test_nft_transfer_not_owner,
    "tokens",
    "nft-transfer-not-owner",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call_response!(
    test_nft_transfer_self,
    "tokens",
    "nft-transfer-self",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::UInt(2));
    }
);

test_contract_call_response!(
    test_nft_transfer_other,
    "tokens",
    "nft-transfer-other",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_unwrap_panic_some,
    "unwrap-panic",
    "unwrap-some",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call!(
    test_unwrap_panic_none,
    "unwrap-panic",
    "unwrap-none",
    |result: Result<Value, Error>| {
        // Expecting a RuntimeErrorType::Panic
        assert!(matches!(result, Err(Error::Wasm(WasmError::Runtime(_)))));
    }
);

test_contract_call_response!(
    test_unwrap_panic_ok,
    "unwrap-panic",
    "unwrap-ok",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::UInt(1));
    }
);

test_contract_call!(
    test_unwrap_panic_error,
    "unwrap-panic",
    "unwrap-error",
    |result: Result<Value, Error>| {
        // Expecting a RuntimeErrorType::Panic
        assert!(matches!(result, Err(Error::Wasm(WasmError::Runtime(_)))));
    }
);

test_contract_call_response!(
    test_map_insert,
    "maps",
    "test-map-insert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_map_insert_exists,
    "maps",
    "test-map-insert-exists",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(false));
    }
);

test_contract_call_response!(
    test_map_set,
    "maps",
    "test-map-set",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_map_set_exists,
    "maps",
    "test-map-set-exists",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_map_get_insert,
    "maps",
    "test-map-get-insert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::some(Value::UInt(2)).unwrap());
    }
);

test_contract_call_response!(
    test_map_get_insert_exists,
    "maps",
    "test-map-get-insert-exists",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::some(Value::UInt(1)).unwrap());
    }
);

test_contract_call_response!(
    test_map_get_set,
    "maps",
    "test-map-get-set",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::some(Value::UInt(2)).unwrap());
    }
);

test_contract_call_response!(
    test_map_get_set_exists,
    "maps",
    "test-map-get-set-exists",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::some(Value::UInt(2)).unwrap());
    }
);

test_contract_call_response!(
    test_map_get_none,
    "maps",
    "test-map-get-none",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::none());
    }
);

test_contract_call_response!(
    test_map_delete,
    "maps",
    "test-map-delete",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract_call_response!(
    test_map_delete_none,
    "maps",
    "test-map-delete-none",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(false));
    }
);

test_contract_call_response!(
    test_map_delete_get,
    "maps",
    "test-map-delete-get",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::none());
    }
);

// These tests are disabled because they require a block to be present in the
// chain, which is not the case when running the tests. Once the test framework
// supports this, these tests can be re-enabled.

// test_contract_call_response!(
//     test_gbi_non_existent,
//     "get-block-info",
//     "non-existent",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(*response.data, Value::none());
//     }
// );

// test_contract_call_response!(
//     test_gbi_bhh,
//     "get-block-info",
//     "get-burnchain-header-hash",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(
//             *response.data,
//             Value::some(Value::buff_from(vec![0u8; 32]).unwrap()).unwrap()
//         );
//     }
// );

// test_contract_call_response!(
//     test_gbi_id_hh,
//     "get-block-info",
//     "get-id-header-hash",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(
//             *response.data,
//             Value::some(Value::buff_from(vec![0u8; 32]).unwrap()).unwrap()
//         );
//     }
// );

// test_contract_call_response!(
//     test_gbi_hh,
//     "get-block-info",
//     "get-header-hash",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(
//             *response.data,
//             Value::some(Value::buff_from(vec![0u8; 32]).unwrap()).unwrap()
//         );
//     }
// );

// test_contract_call_response!(
//     test_gbi_miner_address,
//     "get-block-info",
//     "get-miner-address",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(
//             *response.data,
//             Value::some(StandardPrincipalData::transient().into()).unwrap()
//         );
//     }
// );

// test_contract_call_response!(
//     test_gbi_time,
//     "get-block-info",
//     "get-time",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(*response.data, Value::some(Value::UInt(42)).unwrap());
//     }
// );

// test_contract_call_response!(
//     test_gbi_block_reward,
//     "get-block-info",
//     "get-block-reward",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(*response.data, Value::some(Value::UInt(0)).unwrap());
//     }
// );

// test_contract_call_response!(
//     test_gbi_miner_spend_total,
//     "get-block-info",
//     "get-miner-spend-total",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(*response.data, Value::some(Value::UInt(0)).unwrap());
//     }
// );

// test_contract_call_response!(
//     test_gbi_miner_spend_winner,
//     "get-block-info",
//     "get-miner-spend-winner",
//     |response: ResponseData| {
//         assert!(response.committed);
//         assert_eq!(*response.data, Value::some(Value::UInt(0)).unwrap());
//     }
// );
