use clar2wasm::compile;
use clar2wasm_tests::datastore::{BurnDatastore, Datastore, StacksConstants};
use clarity::vm::analysis::{run_analysis, AnalysisDatabase};
use clarity::vm::ast::build_ast_with_diagnostics;
use clarity::vm::clarity_wasm::{call_function, initialize_contract};
use clarity::vm::database::MemoryBackingStore;
use clarity::vm::{eval_all, CallStack, Environment, Value};
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

    let contract_str = std::fs::read_to_string("contracts/fold-bench.clar").unwrap();
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
        .expect("Failed to initialize contract");

        let mut call_stack = CallStack::new();

        let list = Value::list_from((1..=8192).map(Value::Int).collect())
            .expect("failed to construct list argument");

        let result = call_function(
            "fold-add-square",
            &[list.clone(), Value::Int(1)],
            &mut global_context,
            &contract_context,
            &mut call_stack,
            Some(StandardPrincipalData::transient().into()),
            Some(StandardPrincipalData::transient().into()),
            None,
        )
        .expect("Function call failed");
        assert_eq!(result.expect_result_ok().expect_i128(), 183285493761);

        c.bench_function("wasm_fold_add_square", |b| {
            b.iter(|| {
                let _result = call_function(
                    "fold-add-square",
                    &[list.clone(), Value::Int(1)],
                    &mut global_context,
                    &contract_context,
                    &mut call_stack,
                    Some(StandardPrincipalData::transient().into()),
                    Some(StandardPrincipalData::transient().into()),
                    None,
                )
                .expect("Function call failed");
            })
        });
    }

    global_context.commit().unwrap();
}

fn interp_fold_add_square(c: &mut Criterion) {
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
    let mut cost_tracker = LimitedCostTracker::new_free();
    let mut contract_context = ContractContext::new(contract_id.clone(), ClarityVersion::latest());

    let contract_str = std::fs::read_to_string("contracts/fold-bench.clar").unwrap();

    // Parse the contract
    let (mut ast, _, success) = build_ast_with_diagnostics(
        &contract_id,
        &contract_str,
        &mut cost_tracker,
        ClarityVersion::latest(),
        StacksEpochId::latest(),
    );

    if !success {
        panic!("Failed to parse contract");
    }

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(&mut clarity_store);

    // Run the analysis passes
    let mut contract_analysis = run_analysis(
        &contract_id,
        &mut ast.expressions,
        &mut analysis_db,
        false,
        cost_tracker,
        StacksEpochId::latest(),
        ClarityVersion::latest(),
    )
    .expect("Failed to run analysis");

    let mut global_context = GlobalContext::new(
        false,
        CHAIN_ID_TESTNET,
        conn,
        contract_analysis.cost_track.take().unwrap(),
        StacksEpochId::latest(),
    );

    global_context.begin();

    {
        // Initialize the contract
        eval_all(
            &ast.expressions,
            &mut contract_context,
            &mut global_context,
            None,
        )
        .expect("Failed to interpret the contract");

        let func = contract_context
            .lookup_function("fold-add-square")
            .expect("failed to lookup function");

        let mut call_stack = CallStack::new();
        let mut env = Environment::new(
            &mut global_context,
            &contract_context,
            &mut call_stack,
            Some(StandardPrincipalData::transient().into()),
            Some(StandardPrincipalData::transient().into()),
            None,
        );

        let list = Value::list_from((1..=8192).map(Value::Int).collect())
            .expect("failed to construct list argument");

        // Run once outside of benchmark to test the result
        let result = func
            .execute_apply(&[list.clone(), Value::Int(1)], &mut env)
            .expect("Function call failed");
        assert_eq!(result.expect_result_ok().expect_i128(), 183285493761);

        c.bench_function("interp_fold_add_square", |b| {
            b.iter(|| {
                let _result = func
                    .execute_apply(&[list.clone(), Value::Int(1)], &mut env)
                    .expect("Function call failed");
            })
        });
    }

    global_context.commit().unwrap();
}

criterion_group!(
    fold_add_square,
    wasm_fold_add_square,
    interp_fold_add_square
);
criterion_main!(fold_add_square);
