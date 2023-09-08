use clarity::vm::Value;
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::{
    clarity::{ast::ContractAST, ClarityVersion},
    repl::{ClarityCodeSource, ClarityContract, ContractDeployer, Session, SessionSettings},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::borrow::BorrowMut;
use wasmtime::{
    AsContextMut, Caller, Config, Engine, Extern, ExternRef, Func, Instance, Linker, Module, Store,
    Val,
};

/// Load the standard library into a Wasmtime instance. This is used to load in
/// the standard.wat file and link in all of the host interface functions.
pub(crate) fn load_stdlib() -> Result<(Instance, Store<()>), wasmtime::Error> {
    let standard_lib = include_str!("../src/standard/standard.wat");
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let mut linker = Linker::new(&engine);

    // Link in the host interface functions.
    linker
        .func_wrap(
            "clarity",
            "define_function",
            |_: Caller<'_, ()>, _kind: i32, _name_offset: i32, _name_length: i32| {
                println!("define-function");
            },
        )
        .unwrap();

    linker
        .func_wrap(
            "clarity",
            "define_variable",
            |_: Caller<'_, ()>,
             _name_offset: i32,
             _name_length: i32,
             _value_offset: i32,
             _value_length: i32| {
                println!("define-data-var");
            },
        )
        .unwrap();

    linker
        .func_wrap(
            "clarity",
            "get_variable",
            |_: Caller<'_, ()>,
             _name_offset: i32,
             _name_length: i32,
             _return_offset: i32,
             _return_length: i32| {
                println!("var-get");
            },
        )
        .unwrap();

    linker
        .func_wrap(
            "clarity",
            "set_variable",
            |_: Caller<'_, ()>,
             _name_offset: i32,
             _name_length: i32,
             _value_offset: i32,
             _value_length: i32| {
                println!("var-set");
            },
        )
        .unwrap();

    // Create a log function for debugging.
    linker
        .func_wrap("", "log", |_: Caller<'_, ()>, param: i64| {
            println!("log: {param}");
        })
        .unwrap();

    let module = Module::new(&engine, standard_lib).unwrap();
    let instance = linker.instantiate(&mut store, &module)?;
    Ok((instance, store))
}

fn add(c: &mut Criterion) {
    c.bench_function("add: clarity wasm", |b| {
        let (instance, mut store) = load_stdlib().unwrap();
        let add = instance.get_func(store.borrow_mut(), "add-int").unwrap();

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
    // Setup the session with the Clarity contract first
    let mut session = Session::new(SessionSettings::default());
    let contract_source = r#"
(define-read-only (add (x int) (y int))
    (+ x y)
)
    "#
    .to_string();

    let contract = ClarityContract {
        name: "add".to_string(),
        code_source: ClarityCodeSource::ContractInMemory(contract_source),
        clarity_version: ClarityVersion::Clarity2,
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
            "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.add add 42 12345)"
                .to_string(),
            None,
            false,
        )
        .unwrap();

    c.bench_function("add: clarity", |b| {
        b.iter(|| {
            session
                .eval(
                    "(contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.add add 42 12345)"
                        .to_string(),
                    None,
                    false,
                )
                .unwrap();
        })
    });
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
                            Some(ExternRef::new(Value::Int(int_a.checked_add(*int_b).unwrap())))
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

criterion_group!(add_comparison, add, add_externfunc, rust_add, clarity_add);
criterion_main!(add_comparison);
