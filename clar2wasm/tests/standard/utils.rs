use proptest::prelude::*;
use std::{cell::RefCell, ops::DerefMut};
use wasmtime::Val;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PropInt(u128);

impl PropInt {
    pub const fn new(n: u128) -> Self {
        Self(n)
    }

    pub const fn high(&self) -> i64 {
        (self.0 >> 64) as i64
    }

    pub const fn low(&self) -> i64 {
        self.0 as i64
    }
}

impl From<PropInt> for u128 {
    fn from(p: PropInt) -> u128 {
        p.0 as u128
    }
}

impl From<PropInt> for i128 {
    fn from(p: PropInt) -> i128 {
        p.0 as i128
    }
}

/// Convenience trait to unify the result handling of different return values
pub(crate) trait FromWasmResult {
    fn from_wasm_result(v: &[Val]) -> Self;
    fn relevant_slice(s: &mut [Val]) -> &mut [Val];
}

impl FromWasmResult for u128 {
    fn from_wasm_result(v: &[Val]) -> Self {
        match v {
            &[Val::I64(lo), Val::I64(hi)] => ((lo as u64) as u128) | ((hi as u64) as u128) << 64,
            _ => panic!("invalid wasm result"),
        }
    }

    fn relevant_slice(s: &mut [Val]) -> &mut [Val] {
        &mut s[..2]
    }
}

impl FromWasmResult for i128 {
    fn from_wasm_result(v: &[Val]) -> Self {
        u128::from_wasm_result(v) as i128
    }

    fn relevant_slice(s: &mut [Val]) -> &mut [Val] {
        &mut s[..2]
    }
}

impl FromWasmResult for bool {
    fn from_wasm_result(v: &[Val]) -> Self {
        match v {
            &[Val::I32(0), ..] => false,
            &[Val::I32(1), ..] => true,
            _ => panic!("invalid wasm result"),
        }
    }

    fn relevant_slice(s: &mut [Val]) -> &mut [Val] {
        &mut s[..1]
    }
}

prop_compose! {
    pub(crate) fn int128()(n in any::<u128>()) -> PropInt {
        PropInt::new(n)
    }
}

pub(crate) fn test_export_two_args<N, M, R, C>(name: &str, closure: C)
where
    N: From<PropInt>,
    M: From<PropInt>,
    R: FromWasmResult + PartialEq + std::fmt::Debug,
    C: Fn(N, M) -> R,
{
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let fun = instance
        .get_func(store.borrow_mut().deref_mut(), name)
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let res_slice = R::relevant_slice(&mut res);

        fun.call(
            store.borrow_mut().deref_mut(),
            &[n.low().into(), n.high().into(), m.low().into(), m.high().into()],
            res_slice,
        ).expect(&format!("Could not call exported function {name}"));

        let rust_result = closure(n.into(), m.into());
        let wasm_result = R::from_wasm_result(res_slice);

        prop_assert_eq!(rust_result, wasm_result);
    });
}

pub(crate) fn test_export_two_args_checked<N, M, R, C>(name: &str, closure: C)
where
    N: From<PropInt>,
    M: From<PropInt>,
    R: FromWasmResult + PartialEq + std::fmt::Debug,
    C: Fn(N, M) -> Option<R>,
{
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let fun = instance
        .get_func(store.borrow_mut().deref_mut(), name)
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];

        let call = fun.call(
            store.borrow_mut().deref_mut(),
            &[n.low().into(), n.high().into(), m.low().into(), m.high().into()],
            &mut res,
        );

        match closure(n.into(), m.into()) {
            Some(rust_result) => {
                call.expect(&format!("call to {name} failed"));
                let wasm_result = R::from_wasm_result(&res);
                prop_assert_eq!(rust_result, wasm_result);
            },
            None => { call.expect_err("expected error"); }
        }
    });
}

pub(crate) fn test_export_one_arg<N, R, C>(name: &str, closure: C)
where
    N: From<PropInt>,
    R: FromWasmResult + PartialEq + std::fmt::Debug,
    C: Fn(N) -> R,
{
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let fun = instance
        .get_func(store.borrow_mut().deref_mut(), name)
        .unwrap();

    proptest!(|(n in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let res_slice = R::relevant_slice(&mut res);

        fun.call(
            store.borrow_mut().deref_mut(),
            &[n.low().into(), n.high().into()],
            res_slice,
        ).expect(&format!("Could not call exported function {name}"));

        let rust_result = closure(n.into());
        let wasm_result = R::from_wasm_result(res_slice);

        prop_assert_eq!(rust_result, wasm_result);
    });
}

pub(crate) fn test_export_one_arg_checked<N, R, C>(name: &str, closure: C)
where
    N: From<PropInt>,
    R: FromWasmResult + PartialEq + std::fmt::Debug,
    C: Fn(N) -> Option<R>,
{
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let fun = instance
        .get_func(store.borrow_mut().deref_mut(), name)
        .unwrap();

    proptest!(|(n in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];

        let call = fun.call(
            store.borrow_mut().deref_mut(),
            &[n.low().into(), n.high().into()],
            &mut res,
        );

        match closure(n.into()) {
            Some(rust_result) => {
                call.expect(&format!("call to {name} failed"));
                let wasm_result = R::from_wasm_result(&res);
                prop_assert_eq!(rust_result, wasm_result);
            },
            None => { call.expect_err("expected error"); }
        }
    });
}
