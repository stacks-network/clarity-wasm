use clarity::{
    consts::CHAIN_ID_TESTNET,
    types::StacksEpochId,
    vm::{
        clarity_wasm::initialize_contract,
        contexts::GlobalContext,
        costs::LimitedCostTracker,
        database::{ClarityDatabase, MemoryBackingStore},
        types::{QualifiedContractIdentifier, StandardPrincipalData},
        ClarityVersion, ContractContext, Value,
    },
};

use crate::{
    compile,
    datastore::{BurnDatastore, StacksConstants},
};

pub fn evaluate_at(snippet: &str, epoch: StacksEpochId, version: ClarityVersion) -> Option<Value> {
    let constants = StacksConstants::default();
    let burn_datastore = BurnDatastore::new(constants);
    let mut clarity_store = MemoryBackingStore::new();
    let cost_tracker = LimitedCostTracker::new_free();

    let mut db = ClarityDatabase::new(&mut clarity_store, &burn_datastore, &burn_datastore);
    db.begin();
    db.set_clarity_epoch_version(epoch);
    db.commit();

    let contract_id =
        QualifiedContractIdentifier::new(StandardPrincipalData::transient(), "contract".into());

    let mut compile_result = clarity_store
        .as_analysis_db()
        .execute(|analysis_db| {
            compile(
                snippet,
                &contract_id,
                LimitedCostTracker::new_free(),
                version,
                epoch,
                analysis_db,
            )
        })
        .expect("Failed to compile contract.");

    clarity_store
        .as_analysis_db()
        .execute(|analysis_db| {
            analysis_db.insert_contract(&contract_id, &compile_result.contract_analysis)
        })
        .expect("Failed to insert contract analysis.");

    let mut contract_context = ContractContext::new(contract_id.clone(), version);
    contract_context.set_wasm_module(compile_result.module.emit_wasm());

    let mut global_context = GlobalContext::new(
        false,
        CHAIN_ID_TESTNET,
        clarity_store.as_clarity_db(),
        cost_tracker,
        epoch,
    );
    global_context.begin();
    global_context
        .execute(|g| g.database.insert_contract_hash(&contract_id, snippet))
        .expect("Failed to insert contract hash.");

    initialize_contract(
        &mut global_context,
        &mut contract_context,
        None,
        &compile_result.contract_analysis,
    )
    .expect("Failed to initialize contract.")
}

pub fn evaluate(snippet: &str) -> Option<Value> {
    evaluate_at(snippet, StacksEpochId::latest(), ClarityVersion::latest())
}

#[test]
fn test_evaluate_snippet() {
    let result = evaluate("(+ 1 2)");
    assert_eq!(result, Some(Value::Int(3)));
}
