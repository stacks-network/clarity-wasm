use clarity::{
    consts::CHAIN_ID_TESTNET,
    types::StacksEpochId,
    vm::{
        contexts::GlobalContext,
        costs::LimitedCostTracker,
        database::ClarityDatabase,
        types::{QualifiedContractIdentifier, StandardPrincipalData},
        ClarityVersion, ContractContext, ContractName,
    },
};
use wasmtime::Val;

use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm_tests::{ClarityWasmResult, WasmtimeHelper};

type Res = Option<Box<ClarityWasmResult>>;

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
            let constants = StacksConstants {
                burn_start_height: 0,
                pox_prepare_length: 0,
                pox_reward_cycle_length: 0,
                pox_rejection_fraction: 0,
                epoch_21_start_height: 0,
            };
            let burn_datastore = BurnDatastore::new(constants);
            let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
            conn.begin();
            conn.set_clarity_epoch_version(StacksEpochId::Epoch24);
            conn.commit();
            let cost_tracker = LimitedCostTracker::new_free();
            let mut global_context = GlobalContext::new(
                false,
                CHAIN_ID_TESTNET,
                conn,
                cost_tracker,
                StacksEpochId::Epoch24,
            );
            let mut contract_context =
                ContractContext::new(contract_id.clone(), ClarityVersion::Clarity2);

            global_context.begin();

            {
                let mut helper = WasmtimeHelper::new_from_file(
                    contract_id,
                    &mut global_context,
                    &mut contract_context,
                );

                if let ClarityWasmResult::Response {
                    indicator,
                    ok_value,
                    err_value,
                } = helper.call_public_function($contract_func, $params)
                {
                    $test(indicator, ok_value, err_value);
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

test_contract!(
    test_add,
    "add",
    "simple",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 3);
        }
    }
);

test_contract!(
    test_call_private_with_args_nested,
    "call-private-with-args",
    "call-it",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 3);
        }
    }
);

test_contract!(
    test_call_public,
    "call-public",
    "simple",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 42);
        }
    }
);

test_contract!(
    test_call_public_nested,
    "call-public",
    "call-it",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 42);
        }
    }
);

test_contract!(
    test_call_public_with_args,
    "call-public-with-args",
    "simple",
    &[Val::I64(0), Val::I64(20), Val::I64(0), Val::I64(22)],
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 42);
        }
    }
);

test_contract!(
    test_call_public_with_args_nested,
    "call-public-with-args",
    "call-it",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 3);
        }
    }
);

test_contract!(
    test_define_public_err,
    "define-public-err",
    "simple",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 0);
        assert!(ok_value.is_none());
        assert!(err_value.is_some());
        if let ClarityWasmResult::Int { high, low } = *err_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 42);
        }
    }
);

test_contract!(
    test_define_public_ok,
    "define-public-ok",
    "simple",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 42);
        }
    }
);

test_contract!(
    test_var_get,
    "var-get",
    "simple",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 123);
        }
    }
);

test_contract!(
    test_var_set,
    "var-set",
    "simple",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0x123);
            assert_eq!(low, 0x456);
        }
    }
);

test_contract!(
    test_fold,
    "fold",
    "fold-sub",
    |indicator, ok_value: Res, err_value: Res| {
        assert_eq!(indicator, 1);
        assert!(ok_value.is_some());
        assert!(err_value.is_none());
        if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
            assert_eq!(high, 0);
            assert_eq!(low, 2);
        }
    }
);
