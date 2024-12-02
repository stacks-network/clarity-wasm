#![allow(clippy::expect_used, clippy::unwrap_used)]

use clar2wasm::compile;
use clar2wasm::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm::initialize::initialize_contract;
use clarity::consts::CHAIN_ID_TESTNET;
use clarity::types::StacksEpochId;
use clarity::vm::analysis::{run_analysis, AnalysisDatabase};
use clarity::vm::ast::build_ast_with_diagnostics;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::{ClarityDatabase, MemoryBackingStore};
use clarity::vm::types::{
    BuffData, QualifiedContractIdentifier, SequenceData, StandardPrincipalData,
};
use clarity::vm::{
    eval_all, CallStack, ClarityVersion, ContractContext, ContractName, Environment, Value,
};
use criterion::{criterion_group, criterion_main, Criterion};
use paste::paste;
use pprof::criterion::{Output, PProfProfiler};

fn interpreter(c: &mut Criterion, fn_name: &str, clarity: &str, args: &[Value]) {
    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from(format!("clarity-{}", fn_name).as_str()),
    );
    let mut datastore = Datastore::new();
    let constants = StacksConstants::default();
    let burn_datastore = BurnDatastore::new(constants);
    let mut clarity_store = MemoryBackingStore::new();
    let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
    conn.begin();
    conn.set_clarity_epoch_version(StacksEpochId::latest())
        .unwrap();
    conn.commit().unwrap();
    let mut cost_tracker: LimitedCostTracker = LimitedCostTracker::new_free();
    let mut contract_context: ContractContext =
        ContractContext::new(contract_id.clone(), ClarityVersion::latest());

    let contract_str = clarity.to_string();

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(&mut clarity_store);

    // Parse the contract
    let (ast, _, success) = build_ast_with_diagnostics(
        &contract_id,
        &contract_str,
        &mut cost_tracker,
        ClarityVersion::latest(),
        StacksEpochId::latest(),
    );

    if !success {
        panic!("Failed to parse contract");
    }

    // Run the analysis passes
    let mut contract_analysis = run_analysis(
        &contract_id,
        &ast.expressions,
        &mut analysis_db,
        false,
        cost_tracker,
        StacksEpochId::latest(),
        ClarityVersion::latest(),
        true,
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

    // Initialize the contract
    eval_all(
        &ast.expressions,
        &mut contract_context,
        &mut global_context,
        None,
    )
    .expect("Failed to initialize the contract");

    let func = contract_context
        .lookup_function(fn_name)
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

    c.bench_function(format!("intp_{}", fn_name).as_str(), |b| {
        b.iter(|| {
            env.execute_function_as_transaction(&func, args, None, false)
                .expect("Function call failed");
        })
    });

    global_context.commit().unwrap();
}

fn wasm(c: &mut Criterion, fn_name: &str, clarity: &str, args: &[Value]) {
    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from(format!("clarity-{}", fn_name).as_str()),
    );
    let mut datastore = Datastore::new();
    let constants = StacksConstants::default();
    let burn_datastore = BurnDatastore::new(constants);
    let mut clarity_store = MemoryBackingStore::new();
    let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
    conn.begin();
    conn.set_clarity_epoch_version(StacksEpochId::latest())
        .unwrap();
    conn.commit().unwrap();
    let cost_tracker: LimitedCostTracker = LimitedCostTracker::new_free();
    let mut contract_context: ContractContext =
        ContractContext::new(contract_id.clone(), ClarityVersion::latest());

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(&mut clarity_store);

    let mut compilation = compile(
        clarity,
        &contract_id,
        cost_tracker,
        ClarityVersion::latest(),
        StacksEpochId::latest(),
        &mut analysis_db,
    )
    .expect("Failed compiling clarity to WASM");

    let mut global_context = GlobalContext::new(
        false,
        CHAIN_ID_TESTNET,
        conn,
        compilation.contract_analysis.cost_track.take().unwrap(),
        StacksEpochId::latest(),
    );

    contract_context.set_wasm_module(compilation.module.emit_wasm());

    global_context.begin();

    // Initialize the contract
    initialize_contract(
        &mut global_context,
        &mut contract_context,
        None,
        &compilation.contract_analysis,
    )
    .expect("Failed to initialize the contract");

    let func = contract_context
        .lookup_function(fn_name)
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

    c.bench_function(format!("wasm_{}", fn_name).as_str(), |b| {
        b.iter(|| {
            env.execute_function_as_transaction(&func, args, None, false)
                .expect("Function call failed");
        })
    });

    global_context.commit().unwrap();
}

macro_rules! decl_benches {
    ($(($fn_name:literal, $clarity:literal, [$($arg:expr),*])),* $(,)?) => {
        paste! {
            $(
                #[allow(non_snake_case)]
                fn [<interpreter _ $fn_name>](c: &mut Criterion) {
                    interpreter(c, $fn_name, $clarity, &[$($arg),*])
                }

                #[allow(non_snake_case)]
                fn [<wasm _ $fn_name>](c: &mut Criterion) {
                    wasm(c, $fn_name, $clarity, &[$($arg),*])
                }
            )*

            criterion_group! {
                name = comparison;
                config = {
                    if cfg!(feature = "flamegraph") {
                        Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
                    } else if cfg!(feature = "pb") {
                        Criterion::default().with_profiler(PProfProfiler::new(100, Output::Protobuf))
                    } else {
                        Criterion::default()
                    }
                };
                targets = $([<interpreter _ $fn_name>], [<wasm _ $fn_name>]),*
            }
            criterion_main!(comparison);
        }
    };
}

decl_benches! {
    (
        "add",
        r#"
         (define-read-only (add (x int) (y int))
             (+ x y)
         )
        "#,
        [Value::Int(42), Value::Int(12345)]
    ),
    (
        "sub",
        r#"
         (define-read-only (sub (x int) (y int))
             (- x y)
         )
        "#,
        [Value::Int(12345), Value::Int(45)]
    ),
    (
        "mul",
        r#"
         (define-read-only (mul (x int) (y int))
             (* x y)
         )
        "#,
        [Value::Int(42), Value::Int(24)]
    ),
    (
        "div",
        r#"
         (define-read-only (div (x int) (y int))
             (/ x y)
         )
        "#,
        [Value::Int(42), Value::Int(3)]
    ),
    (
        "bit_and",
        r#"
         (define-read-only (bit_and (x int) (y int))
             (bit-and x y)
         )
        "#,
        [Value::Int(42), Value::Int(3)]
    ),
    (
        "bit_or",
        r#"
         (define-read-only (bit_or (x int) (y int))
             (bit-or x y)
         )
        "#,
        [Value::Int(42), Value::Int(3)]
    ),
    (
        "bit_not",
        r#"
         (define-read-only (bit_not (x int))
             (bit-not x)
         )
        "#,
        [Value::Int(42)]
    ),
    (
        "bit_shift_left",
        r#"
         (define-read-only (bit_shift_left (x int) (y uint))
             (bit-shift-left x y)
         )
        "#,
        [Value::Int(42), Value::UInt(3)]
    ),
    (
        "bit_shift_right",
        r#"
         (define-read-only (bit_shift_right (x int) (y uint))
             (bit-shift-right x y)
         )
        "#,
        [Value::Int(42), Value::UInt(3)]
    ),
    (
        "bit_xor",
        r#"
         (define-read-only (bit_xor (x int) (y int))
             (bit-xor x y)
         )
        "#,
        [Value::Int(42), Value::Int(3)]
    ),
    (
        "SHA256",
        r#"
         (define-read-only (SHA256 (data (buff 1000)))
             (sha256 data)
         )
        "#,
        [Value::Sequence(SequenceData::Buffer(BuffData {data: vec![42; 1000] }))]
    ),
    (
        "SHA512",
        r#"
         (define-read-only (SHA512 (data (buff 1000)))
             (sha512 data)
         )
        "#,
        [Value::Sequence(SequenceData::Buffer(BuffData {data: vec![42; 1000] }))]
    ),
    (
        "fold_add_square",
        r#"
        (define-private (add_square (x int) (y int))
            (+ (* x x) y)
        )

        (define-public (fold_add_square (l (list 2048 int)) (init int))
            (ok (fold add_square l init))
        )
        "#,
        [Value::cons_list_unsanitized((1..=2048).map(Value::Int).collect()).unwrap(), Value::Int(1)]
    ),
}
