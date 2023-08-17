use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm_tests::WasmtimeHelper;
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
use criterion::{criterion_group, criterion_main, Criterion};

fn fold_add(c: &mut Criterion) {
    c.bench_function("fold_add", |b| {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from("fold-bench"),
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

            b.iter(|| {
                helper.call_public_function("fold-add", &[]);
            });
        }
        global_context.commit().unwrap();
    });
}

criterion_group!(all, fold_add);
criterion_main!(all);
