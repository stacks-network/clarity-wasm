use clar2wasm::compile;
use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clarity::{
    consts::CHAIN_ID_TESTNET,
    types::StacksEpochId,
    vm::{
        clarity_wasm::{call_function, initialize_contract},
        contexts::GlobalContext,
        costs::LimitedCostTracker,
        database::{ClarityDatabase, MemoryBackingStore},
        types::{QualifiedContractIdentifier, ResponseData, StandardPrincipalData},
        ClarityVersion, ContractContext, ContractName, Value,
    },
};

/// This macro provides a convenient way to test functions inside contracts.
/// In order, it takes as parameters:
/// - the name of the test to create,
/// - the name of the contract containing the function,
/// - the name of the function to test,
/// - an optional list of parameters,
/// - a closure with type `|indicator: i32, ok_value: Res, err_value: Res|`, and
///   that contains all the assertions we want to test.
macro_rules! test_contract {
    ($func: ident, $contract_name: literal, $contract_func: literal, $params: expr, $test: expr) => {
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
                    &compile_result.contract_analysis,
                )
                .expect("Failed to initialize contract.");

                let result = call_function(
                    &mut global_context,
                    &mut contract_context,
                    $contract_func,
                    $params,
                )
                .expect("Function call failed.");

                if let Value::Response(response_data) = result {
                    $test(response_data);
                } else {
                    panic!("Unexpected result received from WASM function call.");
                }
            }

            global_context.commit().unwrap();
        }
    };

    ($func: ident, $contract_name: literal, $contract_func: literal, $test: expr) => {
        test_contract!($func, $contract_name, $contract_func, &[], $test);
    };
}

test_contract!(test_add, "add", "simple", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::Int(3));
});

test_contract!(
    test_call_private_with_args_nested,
    "call-private-with-args",
    "call-it",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract!(
    test_call_public,
    "call-public",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract!(
    test_call_public_nested,
    "call-public",
    "call-it",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract!(
    test_call_public_with_args,
    "call-public-with-args",
    "simple",
    &[Value::Int(20), Value::Int(22)],
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract!(
    test_call_public_with_args_nested,
    "call-public-with-args",
    "call-it",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract!(
    test_define_public_err,
    "define-public-err",
    "simple",
    |response: ResponseData| {
        assert!(!response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract!(
    test_define_public_ok,
    "define-public-ok",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(42));
    }
);

test_contract!(
    test_var_get,
    "var-get",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(123));
    }
);

test_contract!(
    test_var_set,
    "var-set",
    "simple",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(0x123_0000_0000_0000_0456));
    }
);

test_contract!(test_fold, "fold", "fold-sub", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::Int(2));
});

test_contract!(test_begin, "begin", "simple", |response: ResponseData| {
    assert!(response.committed);
    assert_eq!(*response.data, Value::Int(7));
});

test_contract!(
    test_less_than,
    "cmp-arith",
    "less-uint",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract!(
    test_greater_or_equal,
    "cmp-arith",
    "greater-or-equal-int",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract!(
    test_bitwise_and,
    "bit-and",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract!(
    test_bitwise_not,
    "bit-not",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(-4));
    }
);

test_contract!(
    test_bitwise_or,
    "bit-or",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract!(
    test_bitwise_shift_left,
    "bit-shift-left",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(6));
    }
);

test_contract!(
    test_bitwise_shift_right,
    "bit-shift-right",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(3));
    }
);

test_contract!(
    test_bitwise_xor,
    "bit-xor",
    "assert",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Int(1));
    }
);

test_contract!(
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
