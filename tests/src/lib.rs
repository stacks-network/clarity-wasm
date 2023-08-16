#[cfg(test)]
mod datastore;
#[cfg(test)]
mod util;

#[cfg(test)]
mod tests {
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

    use crate::{
        datastore::{BurnDatastore, Datastore, StacksConstants},
        util::{ClarityWasmResult, WasmtimeHelper},
    };

    #[test]
    fn add() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("add"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("simple", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 3);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn call_private_with_args_nested() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("call-private-with-args"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("call-it", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 3);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn call_public() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("call-public"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("simple", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 42);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn call_public_nested() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("call-public"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("call-it", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 42);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn call_public_with_args() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("call-public-with-args"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            let params = &[Val::I64(0), Val::I64(20), Val::I64(0), Val::I64(22)];

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("simple", params)
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 42);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn call_public_with_args_nested() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("call-public-with-args"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("call-it", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 3);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn define_public_err() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("define-public-err"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("simple", &[])
            {
                assert_eq!(indicator, 0);
                assert!(ok_value.is_none());
                assert!(err_value.is_some());
                if let ClarityWasmResult::Int { high, low } = *err_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 42);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn define_public_ok() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("define-public-ok"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("simple", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 42);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }

    #[test]
    fn var_get() {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("var-get"),
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
            let mut helper =
                WasmtimeHelper::new(contract_id, &mut global_context, &mut contract_context);

            if let ClarityWasmResult::Response {
                indicator,
                ok_value,
                err_value,
            } = helper.call_public_function("simple", &[])
            {
                assert_eq!(indicator, 1);
                assert!(ok_value.is_some());
                assert!(err_value.is_none());
                if let ClarityWasmResult::Int { high, low } = *ok_value.unwrap() {
                    assert_eq!(high, 0);
                    assert_eq!(low, 123);
                }
            } else {
                panic!("Unexpected result received from WASM function call.");
            }
        }
        global_context.commit().unwrap();
    }
}
