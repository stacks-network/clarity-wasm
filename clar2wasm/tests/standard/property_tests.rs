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
    let store = RefCell::new(store); // Ok to borrow_mut since Store<()>
    let add = instance
        .get_func(store.borrow_mut().deref_mut(), "add-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut sum = [Val::I64(0), Val::I64(0)];
        let res = add.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut sum,
        );
        match n.unsigned().checked_add(m.unsigned()) {
            Some(rust_result) => {
                res.expect("call to add-uint failed");
                let wasm_result = PropInt::from_wasm(sum[0].i64().unwrap(), sum[1].i64().unwrap());
                prop_assert_eq!(rust_result, wasm_result.unsigned());
            }
            None => {res.expect_err("expected overflow");}
        }
    })
}
