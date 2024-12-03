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
use clarity::vm::types::{QualifiedContractIdentifier, StandardPrincipalData};
use clarity::vm::{
    eval_all, CallStack, ClarityVersion, ContractContext, ContractName, Environment, Value,
};
use criterion::measurement::Measurement;
use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use paste::paste;
use pprof::criterion::{Output, PProfProfiler};

fn interpreter<M>(b: &mut Bencher<M>, fn_name: &str, clarity: &str, args: &[Value])
where
    M: 'static + Measurement,
{
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

    b.iter(|| {
        env.execute_function_as_transaction(&func, args, None, false)
            .expect("Function call failed");
    });

    global_context.commit().unwrap();
}

fn webassembly<M>(b: &mut Bencher<M>, fn_name: &str, clarity: &str, args: &[Value])
where
    M: 'static + Measurement,
{
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

    b.iter(|| {
        env.execute_function_as_transaction(&func, args, None, false)
            .expect("Function call failed");
    });

    global_context.commit().unwrap();
}

fn criterion_config() -> Criterion {
    if cfg!(feature = "flamegraph") {
        Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
    } else if cfg!(feature = "pb") {
        Criterion::default().with_profiler(PProfProfiler::new(100, Output::Protobuf))
    } else {
        Criterion::default()
    }
}

/// Used to declare benchmarks of clarity contracts to be run on both the interpreter and the
/// WebAssembly runtime.
/// Each arm should only be matched once and declares a criterion group that can then be picked up
/// by [`criterion_main!`].
macro_rules! decl_benches {
    // single
    ($(($fn_name:literal, $clarity:literal, [$($arg:expr),*])),* $(,)?) => {
        paste! {
            $(
                #[allow(non_snake_case)]
                fn [<single _ $fn_name>](c: &mut Criterion) {
                    let mut group = c.benchmark_group($fn_name);
                    group.bench_function("interpreter", |b| {
                        interpreter(b, $fn_name, $clarity, &[$($arg),*]);
                    });
                    group.bench_function("webassembly", |b| {
                        webassembly(b, $fn_name, $clarity, &[$($arg),*]);
                    });
                }
            )*

            criterion_group! {
                name = single;
                config = criterion_config();
                targets = $([<single _ $fn_name>]),*
            }
        }
    };
    // range
    ($(($fn_name:literal, $range:expr, $closure:expr)),* $(,)?) => {
        paste! {
            $(
                #[allow(non_snake_case)]
                fn [<range _ $fn_name>](c: &mut Criterion) {
                    let mut group = c.benchmark_group($fn_name);

                    let closure = $closure;

                    for i in $range {
                        let (clarity, args) = closure(i);
                        group.bench_with_input(BenchmarkId::new("interpreter", i), &i, |b, _| {
                            interpreter(b, $fn_name, &clarity, &args)
                        });
                        group.bench_with_input(BenchmarkId::new("webassembly", i), &i, |b, _| {
                            webassembly(b, $fn_name, &clarity, &args)
                        });
                    }
                }
            )*

            criterion_group! {
                name = range;
                config = criterion_config();
                targets = $([<range _ $fn_name>]),*
            }
        }
    }
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
}

decl_benches! {
    (
        "fold_add_square",
        (1..=1001).step_by(50),
        |i| {
            let clarity = format!(
            r#"
                (define-private (add_square (x int) (y int))
                    (+ (* x x) y)
                )

                (define-public (fold_add_square (l (list {i} int)) (init int))
                    (ok (fold add_square l init))
                )
            "#);
            let args = [Value::cons_list_unsanitized((1..=i).map(Value::Int).collect()).unwrap(), Value::Int(0)];
            (clarity, args)
        }
    ),
    (
        "map_set_entries",
        (1..=1001).step_by(50),
        |i| {
            let clarity = format!(
            r#"
                (define-map mymap int int)

                (define-public (map_set_entries (l (list {i} int)))
                    (begin
                        (map set_entry l)
                        (ok true)
                    )
                )

                (define-private (set_entry (entry int))
                    (map-set mymap entry entry)
                )
            "#);
            let args = [Value::cons_list_unsanitized((1..=i).map(Value::Int).collect()).unwrap()];
            (clarity, args)
        }
    ),
}

criterion_main!(single, range);
