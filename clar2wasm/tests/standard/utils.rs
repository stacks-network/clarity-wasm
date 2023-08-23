use wasmtime::{Caller, Engine, Instance, Linker, Module, Store};

/// Load the standard library into a Wasmtime instance. This is used to load in
/// the standard.wat file and link in all of the host interface functions.
pub(crate) fn load_stdlib() -> Result<(Instance, Store<()>), wasmtime::Error> {
    let standard_lib = include_str!("../../src/standard/standard.wat");
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let mut linker = Linker::new(&engine);

    // Link in the host interface functions.
    linker
        .func_wrap(
            "clarity",
            "define_variable",
            |_: Caller<'_, ()>,
             identifier: i32,
             _name_offset: i32,
             _name_length: i32,
             _value_offset: i32,
             _value_length: i32| {
                println!("define-data-var: {identifier}");
            },
        )
        .unwrap();

    linker
        .func_wrap(
            "clarity",
            "get_variable",
            |_: Caller<'_, ()>, identifier: i32, _return_offset: i32, _return_length: i32| {
                println!("var-get: {identifier}");
            },
        )
        .unwrap();

    linker
        .func_wrap(
            "clarity",
            "set_variable",
            |_: Caller<'_, ()>, identifier: i32, _return_offset: i32, _return_length: i32| {
                println!("var-set: {identifier}");
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
