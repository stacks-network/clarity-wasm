use std::{cell::RefCell, ops::DerefMut};

use crate::utils::load_stdlib;

use proptest::prelude::*;
use wasmtime::Val;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PropInt(u128);

impl PropInt {
    const fn new(n: u128) -> Self {
        Self(n)
    }

    const fn from_wasm(high: i64, low: i64) -> Self {
        Self(((high as u64) as u128) << 64 | ((low as u64) as u128))
    }

    const fn signed(&self) -> i128 {
        self.0 as i128
    }

    const fn unsigned(&self) -> u128 {
        self.0
    }

    const fn high(&self) -> i64 {
        (self.0 >> 64) as i64
    }

    const fn low(&self) -> i64 {
        self.0 as i64
    }
}

prop_compose! {
    fn int128()(n in any::<u128>()) -> PropInt {
        PropInt::new(n)
    }
}

#[test]
fn prop_add_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let add = instance
        .get_func(store.borrow_mut().deref_mut(), "add-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = add.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.unsigned().checked_add(m.unsigned()) {
            Some(rust_result) => {
                call.expect("call to add-uint failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.unsigned());
            }
            None => { call.expect_err("expected overflow"); }
        }
    })
}

#[test]
fn prop_add_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let add = instance
        .get_func(store.borrow_mut().deref_mut(), "add-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = add.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.signed().checked_add(m.signed()) {
            Some(rust_result) => {
                call.expect("call to add-int failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.signed());
            }
            None => { call.expect_err("expected overflow"); }
        }
    })
}

#[test]
fn prop_sub_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let sub = instance
        .get_func(store.borrow_mut().deref_mut(), "sub-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = sub.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.unsigned().checked_sub(m.unsigned()) {
            Some(rust_result) => {
                call.expect("call to sub-uint failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.unsigned());
            }
            None => { call.expect_err("expected underflow"); }
        }
    })
}

#[test]
fn prop_sub_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let sub = instance
        .get_func(store.borrow_mut().deref_mut(), "sub-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = sub.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.signed().checked_sub(m.signed()) {
            Some(rust_result) => {
                call.expect("call to sub-int failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.signed());
            }
            None => { call.expect_err("expected underflow"); }
        }
    })
}

#[test]
fn prop_mul_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let mul = instance
        .get_func(store.borrow_mut().deref_mut(), "mul-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = mul.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.unsigned().checked_mul(m.unsigned()) {
            Some(rust_result) => {
                call.expect("call to mul-uint failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.unsigned());
            }
            None => { call.expect_err("expected overflow"); }
        }
    })
}

#[test]
fn prop_mul_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let mul = instance
        .get_func(store.borrow_mut().deref_mut(), "mul-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = mul.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.signed().checked_mul(m.signed()) {
            Some(rust_result) => {
                call.expect("call to mul-int failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.signed());
            }
            None => { call.expect_err("expected overflow"); }
        }
    })
}

#[test]
fn prop_div_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let div = instance
        .get_func(store.borrow_mut().deref_mut(), "div-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = div.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.unsigned().checked_div(m.unsigned()) {
            Some(rust_result) => {
                call.expect("call to div-uint failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.unsigned());
            }
            None => { call.expect_err("expected divide by zero"); }
        }
    })
}

#[test]
fn prop_div_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let div = instance
        .get_func(store.borrow_mut().deref_mut(), "div-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = div.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.signed().checked_div(m.signed()) {
            Some(rust_result) => {
                call.expect("call to div-int failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.signed());
            }
            None => { call.expect_err("expected divide by zero"); }
        }
    })
}

#[test]
fn prop_mod_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let modulo = instance
        .get_func(store.borrow_mut().deref_mut(), "mod-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = modulo.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.unsigned().checked_rem(m.unsigned()) {
            Some(rust_result) => {
                call.expect("call to mod-uint failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.unsigned());
            }
            None => { call.expect_err("expected divide by zero"); }
        }
    })
}

#[test]
fn prop_mod_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let modulo = instance
        .get_func(store.borrow_mut().deref_mut(), "mod-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        let call = modulo.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        );
        match n.signed().checked_rem(m.signed()) {
            Some(rust_result) => {
                call.expect("call to div-int failed");
                let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.signed());
            }
            None if m.signed() == 0 => { call.expect_err("expected divide by zero"); }
            None => { call.expect_err("expected overflow"); }
        }
    })
}

#[test]
fn prop_lt_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let lt = instance
        .get_func(store.borrow_mut().deref_mut(), "lt-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I32(0)];
        lt.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to lt-uint failed");
        prop_assert_eq!(n.unsigned() < m.unsigned(), res[0].i32().unwrap() == 1);
    })
}

#[test]
fn prop_lt_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let lt = instance
        .get_func(store.borrow_mut().deref_mut(), "lt-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I32(0)];
        lt.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to lt-int failed");
        prop_assert_eq!(n.signed() < m.signed(), res[0].i32().unwrap() == 1);
    })
}

#[test]
fn prop_gt_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let gt = instance
        .get_func(store.borrow_mut().deref_mut(), "gt-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I32(0)];
        gt.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to gt-uint failed");
        prop_assert_eq!(n.unsigned() > m.unsigned(), res[0].i32().unwrap() == 1);
    })
}

#[test]
fn prop_gt_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let gt = instance
        .get_func(store.borrow_mut().deref_mut(), "gt-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I32(0)];
        gt.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to gt-int failed");
        prop_assert_eq!(n.signed() > m.signed(), res[0].i32().unwrap() == 1);
    })
}
