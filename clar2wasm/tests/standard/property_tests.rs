use crate::utils;

#[test]
fn prop_add_uint() {
    utils::test_export_two_args_checked("add-uint", |a: u128, b: u128| a.checked_add(b))
}

#[test]
fn prop_add_int() {
    utils::test_export_two_args_checked("add-int", |a: i128, b: i128| a.checked_add(b))
}

#[test]
fn prop_sub_uint() {
    utils::test_export_two_args_checked("sub-uint", |a: u128, b: u128| a.checked_sub(b))
}

#[test]
fn prop_sub_int() {
    utils::test_export_two_args_checked("sub-int", |a: i128, b: i128| a.checked_sub(b))
}

#[test]
fn prop_mul_uint() {
    utils::test_export_two_args_checked("mul-uint", |a: u128, b: u128| a.checked_mul(b))
}

#[test]
fn prop_mul_int() {
    utils::test_export_two_args_checked("mul-int", |a: i128, b: i128| a.checked_mul(b))
}

#[test]
fn prop_div_uint() {
    utils::test_export_two_args_checked("div-uint", |a: u128, b: u128| a.checked_div(b))
}

#[test]
fn prop_div_int() {
    utils::test_export_two_args_checked("div-int", |a: i128, b: i128| a.checked_div(b))
}

#[test]
fn prop_mod_uint() {
    utils::test_export_two_args_checked("mod-uint", |a: u128, b: u128| a.checked_rem(b))
}

#[test]
fn prop_mod_int() {
    utils::test_export_two_args_checked("mod-int", |a: i128, b: i128| a.checked_rem(b))
}

#[test]
fn prop_lt_uint() {
    utils::test_export_two_args("lt-uint", |a: u128, b: u128| a < b)
}

#[test]
fn prop_lt_int() {
    utils::test_export_two_args("lt-int", |a: i128, b: i128| a < b);
}

#[test]
fn prop_gt_uint() {
    utils::test_export_two_args("gt-uint", |a: u128, b: u128| a > b);
}

#[test]
fn prop_gt_int() {
    utils::test_export_two_args("gt-int", |a: i128, b: i128| a > b);
}

#[test]
fn prop_le_uint() {
    utils::test_export_two_args("le-uint", |a: u128, b: u128| a <= b);
}

#[test]
fn prop_le_int() {
    utils::test_export_two_args("le-int", |a: i128, b: i128| a <= b);
}

#[test]
fn prop_ge_uint() {
    utils::test_export_two_args("ge-uint", |a: u128, b: u128| a >= b);
}

#[test]
fn prop_ge_int() {
    utils::test_export_two_args("ge-int", |a: i128, b: i128| a >= b);
}

#[test]
fn prop_log2_uint() {
    utils::test_export_one_arg_checked("log2-uint", |a: u128| a.checked_ilog2().map(|u| u as u128))
}

#[test]
fn prop_log2_int() {
    utils::test_export_one_arg_checked("log2-int", |a: i128| a.checked_ilog2().map(|u| u as i128))
}

#[test]
fn prop_sqrti_uint() {
    utils::test_export_one_arg("sqrti-uint", |a: u128| num_integer::Roots::sqrt(&a) as u128)
}

#[test]
fn prop_sqrti_int() {
    utils::test_export_one_arg_checked("sqrti-int", |a: i128| {
        if a > 0 {
            Some(num_integer::Roots::sqrt(&a))
        } else {
            None
        }
    })
}

#[test]
fn prop_bit_and() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_and = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-and")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        bit_and.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to bit-and failed");
		let rust_result = n.unsigned() & m.unsigned();
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.unsigned());

    })
}

#[test]
fn prop_bit_not() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_not = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-not")
        .unwrap();

    proptest!(|(n in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        bit_not.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into()],
            &mut res,
        ).expect("call to bit-not failed");
		let rust_result = !n.unsigned();
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.unsigned());

    })
}

#[test]
fn prop_bit_or() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_or = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-or")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        bit_or.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to bit-or failed");
		let rust_result = n.unsigned() | m.unsigned();
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.unsigned());
    })
}

#[test]
fn prop_bit_shift_left() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_shift_left = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-shift-left")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
		// bit shifts are always mod 128, as per clarity docs
		let m = (m.unsigned() % 128) as i64;

        let mut res = [Val::I64(0), Val::I64(0)];
        bit_shift_left.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), Val::I64(m)],
            &mut res,
        ).expect("call to bit-shift-left failed");
		let rust_result = n.unsigned().wrapping_shl(m as u32);
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.unsigned());

    })
}

#[test]
fn prop_bit_shift_right_uint() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_shift_right_uint = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-shift-right-uint")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
		// bit shifts are always mod 128, as per clarity docs
		let m = (m.unsigned() % 128) as i64;

        let mut res = [Val::I64(0), Val::I64(0)];
        bit_shift_right_uint.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), Val::I64(m)],
            &mut res,
        ).expect("call to bit-shift-right-uint failed");
		let rust_result = n.unsigned().wrapping_shr(m as u32);
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.unsigned());

    })
}

#[test]
fn prop_bit_shift_right_int() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_shift_right_int = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-shift-right-int")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
		// bit shifts are always mod 128, as per clarity docs
		let m = (m.unsigned() % 128) as i64;

		println!("BY modded {}", m);

        let mut res = [Val::I64(0), Val::I64(0)];
        bit_shift_right_int.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), Val::I64(m)],
            &mut res,
        ).expect("call to bit-shift-right-int failed");
		let rust_result = n.signed().wrapping_shr(m as u32);
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.signed());
    })
}

#[test]
fn prop_bit_xor() {
    let (instance, store) = load_stdlib().unwrap();
    let store = RefCell::new(store);
    let bit_xor = instance
        .get_func(store.borrow_mut().deref_mut(), "bit-xor")
        .unwrap();

    proptest!(|(n in int128(), m in int128())| {
        let mut res = [Val::I64(0), Val::I64(0)];
        bit_xor.call(
            store.borrow_mut().deref_mut(),
            &[n.high().into(), n.low().into(), m.high().into(), m.low().into()],
            &mut res,
        ).expect("call to bit-xor failed");
		let rust_result = n.unsigned() ^ m.unsigned();
        let wasm_result = PropInt::from_wasm(res[0].i64().unwrap(), res[1].i64().unwrap());
        prop_assert_eq!(rust_result, wasm_result.unsigned());

    })
}
