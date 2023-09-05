use clar2wasm::compile;
use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clarity::vm::ast::ContractAST;
use clarity::vm::clarity_wasm::{call_function, initialize_contract};
use clarity::vm::database::MemoryBackingStore;
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
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, Session, SessionSettings,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn wasm_fold_add_square(c: &mut Criterion) {
    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from("fold-bench"),
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
    let mut contract_context = ContractContext::new(contract_id.clone(), ClarityVersion::latest());

    let contract_str = std::fs::read_to_string(format!("contracts/{}.clar", "fold-bench")).unwrap();
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
        .expect("Failed to initialize contract");

        c.bench_function("wasm_fold_add_square", |b| {
            b.iter(|| {
                let _result = call_function(
                    &mut global_context,
                    &mut contract_context,
                    "fold-add-square",
                    &[],
                )
                .expect("Function call failed");
            })
        });
    }

    global_context.commit().unwrap();
}

fn interp_fold_add_square(c: &mut Criterion) {
    // Setup the session with the Clarity contract first
    let mut session = Session::new(SessionSettings::default());
    let contract_source = include_str!("../contracts/fold-bench.clar");

    let contract = ClarityContract {
        name: "fold-bench".to_string(),
        code_source: ClarityCodeSource::ContractInMemory(contract_source.to_string()),
        clarity_version: ClarityVersion::latest(),
        epoch: StacksEpochId::latest(),
        deployer: ContractDeployer::Address(
            "ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM".to_string(),
        ),
    };

    let mut ast: Option<ContractAST> = None;
    session
        .deploy_contract(&contract, None, false, None, &mut ast)
        .unwrap();
    session
        .eval(
            "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.fold-bench fold-add-square)"
                .to_string(),
            None,
            false,
        )
        .unwrap();

    c.bench_function("interp_fold_add_square", |b| {
        b.iter(|| {
            session
                .eval(
                    "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.fold-bench fold-add-square)"
                        .to_string(),
                    None,
                    false,
                )
                .unwrap();
        })
    });
}

criterion_group!(
    fold_add_square,
    wasm_fold_add_square,
    interp_fold_add_square
);
criterion_main!(fold_add_square);
