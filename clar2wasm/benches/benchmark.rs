#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::borrow::BorrowMut;

use clar2wasm::datastore::{BurnDatastore, Datastore, StacksConstants};
use clar2wasm::linker::load_stdlib;
use clar2wasm::wasm_generator::END_OF_STANDARD_DATA;
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
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use wasmtime::{
    AsContextMut, Config, Engine, Extern, ExternRef, Func, Instance, Module, Store, Val,
};

fn add(c: &mut Criterion) {
    c.bench_function("add: clarity wasm", |b| {
        let (instance, mut store) = load_stdlib().unwrap();
        let add = instance
            .get_func(store.borrow_mut(), "stdlib.add-int")
            .unwrap();

        b.iter(|| {
            let mut results = [Val::I64(0), Val::I64(0)];
            add.call(
                &mut store.borrow_mut(),
                &[Val::I64(0), Val::I64(42), Val::I64(0), Val::I64(12345)],
                &mut results,
            )
            .unwrap();
        })
    });
}

pub fn add128(a: i128, b: i128) -> i128 {
    a + b
}

fn rust_add(c: &mut Criterion) {
    c.bench_function("add: rust", |b| {
        b.iter(|| {
            black_box(add128(black_box(42), black_box(12345)));
        })
    });
}

fn clarity_add(c: &mut Criterion) {
    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from("clarity-add"),
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

    let contract_str = r#"
(define-read-only (add (x int) (y int))
    (+ x y)
)
    "#
    .to_string();

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

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(&mut clarity_store);

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
            .lookup_function("add")
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

        c.bench_function("add: clarity", |b| {
            b.iter(|| {
                let _result = func
                    .execute_apply(&[Value::Int(42), Value::Int(12345)], &mut env)
                    .expect("Function call failed");
            })
        });
    }

    global_context.commit().unwrap();
}

fn sha512(c: &mut Criterion) {
    c.bench_function("calculate-sha512: clarity wasm", |b| {
        let (instance, mut store) = load_stdlib().unwrap();
        let sha512 = instance
            .get_func(store.borrow_mut(), "stdlib.sha512-buf")
            .unwrap();

        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("Could not find memory");

        let stack_pointer = instance.get_global(&mut store, "stack-pointer").unwrap();
        stack_pointer.set(&mut store, Val::I32(1500)).unwrap();

        let res_offset: i32 = 3000i32;

        let text = b"Hello, World!";
        memory
            .write(&mut store, END_OF_STANDARD_DATA as usize, text)
            .expect("Should be able to write to memory");

        b.iter(|| {
            let mut results = [Val::I64(0), Val::I64(0)];
            sha512
                .call(
                    &mut store.borrow_mut(),
                    &[
                        Val::I32(END_OF_STANDARD_DATA as i32),
                        Val::I32(text.len() as i32),
                        res_offset.into(),
                    ],
                    &mut results,
                )
                .unwrap();
        })
    });
}

fn clarity_sha512(c: &mut Criterion) {
    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from("clarity-sha512"),
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

    let contract_str = r#"
(define-read-only (calculate-sha512 (data (buff 1000)))
    (sha512 data)
)
    "#
    .to_string();

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

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(&mut clarity_store);

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
            .lookup_function("calculate-sha512")
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

        let input: &str = "Hello World!";

        let hex_buffer: Vec<u8> = input.bytes().collect();

        // If you want the result as a single string
        // let hex_string = hex_buffer.join("");

        c.bench_function("calculate-sha512: clarity", |b| {
            b.iter(|| {
                let buffer_clone = hex_buffer.clone(); // Clone the buffer
                let _result = func
                    .execute_apply(
                        &[Value::Sequence(SequenceData::Buffer(BuffData {
                            data: buffer_clone,
                        }))],
                        &mut env,
                    )
                    .expect("Function call failed");
            })
        });
    }

    global_context.commit().unwrap();
}

fn sha256(c: &mut Criterion) {
    c.bench_function("calculate-sha256: clarity wasm", |b| {
        let (instance, mut store) = load_stdlib().unwrap();
        let sha512 = instance
            .get_func(store.borrow_mut(), "stdlib.sha256-buf")
            .unwrap();

        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("Could not find memory");

        let stack_pointer = instance.get_global(&mut store, "stack-pointer").unwrap();
        stack_pointer.set(&mut store, Val::I32(1500)).unwrap();

        let res_offset: i32 = 3000i32;

        let text = b"Hello, World!";
        memory
            .write(&mut store, END_OF_STANDARD_DATA as usize, text)
            .expect("Should be able to write to memory");

        b.iter(|| {
            let mut results = [Val::I32(0), Val::I32(0)];
            sha512
                .call(
                    &mut store.borrow_mut(),
                    &[
                        Val::I32(END_OF_STANDARD_DATA as i32),
                        Val::I32(text.len() as i32),
                        res_offset.into(),
                    ],
                    &mut results,
                )
                .unwrap();
        })
    });
}

fn clarity_sha256(c: &mut Criterion) {
    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from("clarity-sha256"),
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

    let contract_str = r#"
(define-read-only (calculate-sha256 (data (buff 1000)))
    (sha256 data)
)
    "#
    .to_string();

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

    // Create a new analysis database
    let mut analysis_db = AnalysisDatabase::new(&mut clarity_store);

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
            .lookup_function("calculate-sha256")
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

        let input: &str = "Hello World!";

        let hex_buffer: Vec<u8> = input.bytes().collect();

        // If you want the result as a single string
        // let hex_string = hex_buffer.join("");

        c.bench_function("calculate-sha256: clarity", |b| {
            b.iter(|| {
                let buffer_clone = hex_buffer.clone(); // Clone the buffer
                let _result = func
                    .execute_apply(
                        &[Value::Sequence(SequenceData::Buffer(BuffData {
                            data: buffer_clone,
                        }))],
                        &mut env,
                    )
                    .expect("Function call failed");
            })
        });
    }

    global_context.commit().unwrap();
}
use walrus::{FunctionBuilder, ModuleConfig, ValType};

pub fn generate_wasm() -> Vec<u8> {
    // Construct a new Walrus module.
    let config = ModuleConfig::new();
    let mut module = walrus::Module::with_config(config);

    // Import the API definition for `add`.
    let add_ty = module.types.add(
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );
    let (add, _) = module.add_import_func("env", "add", add_ty);

    // Build the `toplevel` function (all of the below)..
    // This function accepts two Externref's as parameters (for add, should be of integer type)
    // but the host function (in main.rs) only handles Value::Int right now.
    // Returns an Externref which is of the same type as the input types.
    let mut top_level = FunctionBuilder::new(
        &mut module.types,
        &[ValType::Externref, ValType::Externref],
        &[ValType::Externref],
    );

    let a = module.locals.add(ValType::Externref);
    let b = module.locals.add(ValType::Externref);

    top_level.func_body().local_get(a).local_get(b).call(add);

    let top_level_fn = top_level.finish(vec![a, b], &mut module.funcs);
    module.exports.add("toplevel", top_level_fn);

    // Compile the module.
    module.emit_wasm()
}

fn add_externfunc(c: &mut Criterion) {
    c.bench_function("add: externfunc", |b| {
        // Generate a wasm module (see `wasm_generator.rs`) which has a `toplevel` function
        // which in turn calls the below defined wrapped function `func`.
        let wasm_bytes = generate_wasm();

        // Initialize config which allows for reference types.
        let mut config = Config::new();
        config.wasm_reference_types(true);

        // Initialize the wasmtime engine.
        let engine = Engine::new(&config).expect("Failed to initialize engine");

        // Initialize the wasmtime store.
        let mut store = Store::new(&engine, ());

        // Load the module generated above.
        let module =
            Module::from_binary(store.engine(), &wasm_bytes).expect("Failed to load module");

        // This defines a HOST function which receives ExternRef values and adds them together, returning the result.
        // This code only handles sunny-day, i.e. if it isn't two `Value::Int(_)`'s it'll blow up. A proper impl.
        // would use a `match` statement and handle Int/UInt accordingly.
        // NOTE: !!!  ExternRef input arguments and return values must be provided as `Options`s.
        let func = Func::wrap(
            store.as_context_mut(),
            |a: Option<ExternRef>, b: Option<ExternRef>| {
                let a = a.unwrap();
                let b = b.unwrap();

                let result = match a.data().downcast_ref::<Value>() {
                    Some(Value::Int(int_a)) => {
                        if let Some(Value::Int(int_b)) = b.data().downcast_ref::<Value>() {
                            Some(ExternRef::new(Value::Int(
                                int_a.checked_add(*int_b).unwrap(),
                            )))
                        } else {
                            panic!("Value type mismatch");
                        }
                    }
                    Some(Value::UInt(uint_a)) => {
                        if let Some(Value::UInt(uint_b)) = b.data().downcast_ref::<Value>() {
                            Some(ExternRef::new(Value::UInt(uint_a + uint_b)))
                        } else {
                            panic!("Value type mismatch");
                        }
                    }
                    _ => panic!("Invalid type..."),
                };

                Ok(result)
            },
        );

        // Create an `Extern` of the `add` function (needed to pass as an imported function in the next step).
        let add = Extern::Func(func);

        // We create a new instance and pass in any imported (host) functions (in this case, only `add`).
        let instance = Instance::new(&mut store, &module, &[add])
            .expect("Couldn't create new module instance");

        // This would be where we prepare to call a contract function. In this case, `toplevel` (as defined)
        // in the WASM generated by `wasm_generator`. We'll pass two Clarity `Value::Int`'s (1, 2) and
        // receive a `Value::Int` back (3).
        let instance_fn = instance
            .get_func(&mut store, "toplevel")
            .expect("Failed to get fn");

        b.iter(|| {
            // Define our output parameters. Note that we're using `Option`s as stated above.
            let results = &mut [
                Val::ExternRef(Some(ExternRef::new(Value::none()))), // Option<ExternRef>
            ];

            // * * * * * * * * * * * * *
            // Call the function using `Int`s.
            // * * * * * * * * * * * * *
            instance_fn
                .call(
                    &mut store,
                    &[
                        Val::ExternRef(Some(ExternRef::new(Value::Int(1)))), // Option<ExternRef>
                        Val::ExternRef(Some(ExternRef::new(Value::Int(2)))), // Option<ExternRef>
                    ],
                    results,
                )
                .expect("Failed to call function");
        });
    });
}

criterion_group! {
    name = add_comparison;
    config = {
        if cfg!(feature = "flamegraph") {
            Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
        } else if cfg!(feature = "pb") {
            Criterion::default().with_profiler(PProfProfiler::new(100, Output::Protobuf))
        } else {
            Criterion::default()
        }
    };
    targets = add, add_externfunc, rust_add, clarity_add, sha512, clarity_sha512, sha256, clarity_sha256
}
criterion_main!(add_comparison);
