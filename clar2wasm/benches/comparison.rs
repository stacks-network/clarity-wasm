#![allow(clippy::expect_used, clippy::unwrap_used)]

use clar2wasm::compile;
use clar2wasm::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm::initialize::initialize_contract;
use clarity::consts::CHAIN_ID_TESTNET;
use clarity::types::{PrivateKey, StacksEpochId};
use clarity::util::hash::Keccak256Hash;
use clarity::util::secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey};
use clarity::vm::analysis::{run_analysis, AnalysisDatabase};
use clarity::vm::ast::build_ast_with_diagnostics;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::{ClarityDatabase, MemoryBackingStore};
use clarity::vm::types::{QualifiedContractIdentifier, StandardPrincipalData, TupleData};
use clarity::vm::{
    eval_all, CallStack, ClarityName, ClarityVersion, ContractContext, ContractName, Environment,
    Value,
};
use criterion::measurement::Measurement;
use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use paste::paste;
use pprof::criterion::{Output, PProfProfiler};

fn interpreter<M, F>(b: &mut Bencher<M>, fn_name: &str, clarity: &str, init: F)
where
    M: 'static + Measurement,
    F: FnOnce(&mut Environment) -> Vec<Value>,
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

    let args = init(&mut env);

    b.iter(|| {
        env.execute_function_as_transaction(&func, &args, None, false)
            .expect("Function call failed");
    });

    global_context.commit().unwrap();
}

fn webassembly<M, F>(b: &mut Bencher<M>, fn_name: &str, clarity: &str, init: F)
where
    M: 'static + Measurement,
    F: FnOnce(&mut Environment) -> Vec<Value>,
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

    let args = init(&mut env);

    b.iter(|| {
        env.execute_function_as_transaction(&func, &args, None, false)
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
                        interpreter(b, $fn_name, $clarity, |_| vec![$($arg),*]);
                    });
                    group.bench_function("webassembly", |b| {
                        webassembly(b, $fn_name, $clarity, |_| vec![$($arg),*]);
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
    ($(($fn_name:literal, $range:expr, $produce_clarity:expr, $init:expr)),* $(,)?) => {
        paste! {
            $(
                #[allow(non_snake_case)]
                fn [<range _ $fn_name>](c: &mut Criterion) {
                    let mut group = c.benchmark_group($fn_name);

                    let produce_clarity = $produce_clarity;

                    for i in $range {
                        let clarity = produce_clarity(i);
                        group.bench_with_input(BenchmarkId::new("interpreter", i), &i, |b, _| {
                            interpreter(b, $fn_name, &clarity, |env| { $init(i, env) })
                        });
                        group.bench_with_input(BenchmarkId::new("webassembly", i), &i, |b, _| {
                            webassembly(b, $fn_name, &clarity, |env| { $init(i, env) })
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
        |i| format!(r#"
        (define-private (add_square (x int) (y int))
            (+ (* x x) y)
        )

        (define-public (fold_add_square (l (list {i} int)) (init int))
            (ok (fold add_square l init))
        )
        "#),
        |i, _: &mut Environment| vec![Value::cons_list_unsanitized((1..=i).map(Value::Int).collect()).unwrap(), Value::Int(0)]
    ),
    (
        "map_set_entries",
        (1..=1001).step_by(50),
        |i| format!(r#"
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
        "#),
        |i, _: &mut Environment| vec![Value::cons_list_unsanitized((1..=i).map(Value::Int).collect()).unwrap()]
    ),
    (
        "add_prices",
        1..1000,
        |i| format!(r#"
        (define-map oracle_data
            {{ source: uint, symbol: uint }}
            {{ amount: uint }}
        )

        (define-map oracle_sources
            {{ source: uint }}
            {{ key: (buff 33) }}
        )

        (define-read-only (slice-16 (b (buff 48)) (start uint))
            (unwrap-panic (as-max-len? (unwrap-panic (slice? b start (+ start u16))) u16))
        )

        (define-read-only (extract-source (msg (buff 48)))
            (buff-to-uint-le (slice-16 msg u0))
        )

        (define-read-only (extract-symbol (msg (buff 48)))
            (buff-to-uint-le (slice-16 msg u16))
        )

        (define-read-only (extract-amount (msg (buff 48)))
            (buff-to-uint-le (slice-16 msg u32))
        )

        (define-read-only (verify-signature (msg (buff 48)) (signature (buff 65)) (key (buff 33)))
            (is-eq (unwrap-panic (secp256k1-recover? (keccak256 msg) signature)) key)
        )

        (define-private (add-source (source uint) (key (buff 33)))
            (begin
                (map-set oracle_sources {{ source: source }} {{ key: key }})
                (ok true)
            )
        )

        (define-private (add-price (msg (buff 48)) (sig (buff 65)))
            (let ((source (extract-source msg))) 
                (if (verify-signature msg sig (get key (unwrap-panic (map-get? oracle_sources {{source: source}}))))
                    (let ((symbol (extract-symbol msg)) (amount (extract-amount msg)) (data-opt (map-get? oracle_data {{source: source, symbol: symbol}})))
                        (if (is-some data-opt)
                            (let ((data (unwrap-panic data-opt)))
                                (begin 
                                    (map-set oracle_data {{source: source, symbol: symbol}} {{amount: amount}})
                                    (ok true)
                                )
                            )
                            (begin 
                                (map-set oracle_data {{source: source, symbol: symbol}} {{amount: amount}})
                                (ok true)
                            )
                        )
                    )
                    (err u62)
                )
            )
        )

        (define-private (call-add-source (source {{source: uint, key: (buff 33)}}))
            (unwrap-panic (add-source (get source source) (get key source)))
        )

        (define-private (call-add-price (price {{msg: (buff 48), sig: (buff 65)}}))
            (unwrap-panic (add-price (get msg price) (get sig price)))
        )

        (define-public (add_sources (sources (list {i} {{source: uint, key: (buff 33)}})))
            (begin
                (map call-add-source sources)
                (ok true)
            )
        )

        (define-public (add_prices (prices (list {i} {{msg: (buff 48), sig: (buff 65)}})))
            (begin
                (map call-add-price prices)
                (ok true)
            )
        )
        "#),
        |i, env: &mut Environment| vec![add_prices_init(i, env)]
    ),
}

fn add_prices_init(n: usize, env: &mut Environment) -> Value {
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    let mut seed = [0u8; 32];

    let mut rng = StdRng::from_seed(seed);

    let mut sources = Vec::with_capacity(n);
    let mut prices = Vec::with_capacity(n);

    for _ in 0..n {
        rng.fill_bytes(&mut seed);

        let sk = Secp256k1PrivateKey::from_seed(&seed);
        let pk = Secp256k1PublicKey::from_private(&sk);

        let source = rand::random::<u128>();
        let symbol = rand::random::<u128>();
        let amount = rand::random::<u128>();

        let mut msg = [0; 48];
        msg[0..16].copy_from_slice(&source.to_le_bytes());
        msg[16..32].copy_from_slice(&symbol.to_le_bytes());
        msg[32..48].copy_from_slice(&amount.to_le_bytes());

        let msg_hash = Keccak256Hash::from_data(&msg);
        let sig = sk.sign(msg_hash.as_bytes()).unwrap();

        sources.push(Value::Tuple(
            TupleData::from_data(vec![
                (ClarityName::from("source"), Value::UInt(source)),
                (
                    ClarityName::from("key"),
                    Value::buff_from(pk.to_bytes_compressed()).unwrap(),
                ),
            ])
            .unwrap(),
        ));

        prices.push(Value::Tuple(
            TupleData::from_data(vec![
                (
                    ClarityName::from("msg"),
                    Value::buff_from(msg.to_vec()).unwrap(),
                ),
                (
                    ClarityName::from("sig"),
                    Value::buff_from(sig.0.to_vec()).unwrap(),
                ),
            ])
            .unwrap(),
        ));
    }

    let func = env
        .contract_context
        .lookup_function("add_sources")
        .expect("failed to lookup function");

    env.execute_function_as_transaction(
        &func,
        &[Value::cons_list_unsanitized(sources).unwrap()],
        None,
        false,
    )
    .expect("Adding sources should succeed");

    Value::cons_list_unsanitized(prices).unwrap()
}

criterion_main!(single, range);
