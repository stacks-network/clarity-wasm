use clarity::{
    consts::CHAIN_ID_TESTNET,
    types::StacksEpochId,
    vm::{
        contexts::GlobalContext,
        costs::LimitedCostTracker,
        database::ClarityDatabase,
        types::{QualifiedContractIdentifier, ResponseData, StandardPrincipalData},
        ClarityVersion, ContractContext, ContractName, Value,
    },
};
use wasmtime::Val;

use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm_tests::WasmtimeHelper;

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

                if let Value::Response(response_data) =
                    helper.call_public_function($contract_func, $params)
                {
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
    &[Val::I64(0), Val::I64(20), Val::I64(0), Val::I64(22)],
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
    test_greater_than,
    "cmp-arith",
    "greater-int",
    |response: ResponseData| {
        assert!(response.committed);
        assert_eq!(*response.data, Value::Bool(true));
    }
);

test_contract!(
    test_less_or_equal,
    "cmp-arith",
    "less-or-equal-uint",
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
