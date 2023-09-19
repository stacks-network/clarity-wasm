use wasmtime::Val;

use crate::utils::load_stdlib;

#[test]
fn test_add_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let add = instance.get_func(&mut store, "add-uint").unwrap();
    let mut sum = [Val::I64(0), Val::I64(0)];

    // 0 + 0 = 0
    add.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut sum,
    )
    .expect("call to add-uint failed");
    assert_eq!(sum[0].i64(), Some(0));
    assert_eq!(sum[1].i64(), Some(0));

    // 1 + 2 = 3
    add.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut sum,
    )
    .expect("call to add-uint failed");
    assert_eq!(sum[0].i64(), Some(3));
    assert_eq!(sum[1].i64(), Some(0));

    // Carry
    // 0xffff_ffff_ffff_ffff + 1 = 0x1_0000_0000_0000_0000
    add.call(
        &mut store,
        &[Val::I64(-1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut sum,
    )
    .expect("call to add-uint failed");
    assert_eq!(sum[0].i64(), Some(0));
    assert_eq!(sum[1].i64(), Some(1));

    // Overflow
    // 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff + 1 = Overflow
    add.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut sum,
    )
    .expect_err("expected overflow");

    // Overflow
    // 1 + 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff = Overflow
    add.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut sum,
    )
    .expect_err("expected overflow");
}

#[test]
fn test_add_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let add = instance.get_func(&mut store, "add-int").unwrap();
    let mut sum = [Val::I64(0), Val::I64(0)];

    // 0 + 0 = 0
    add.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut sum,
    )
    .expect("call to add-int failed");
    assert_eq!(sum[0].i64(), Some(0));
    assert_eq!(sum[1].i64(), Some(0));

    // 1 + 2 = 3
    add.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut sum,
    )
    .expect("call to add-int failed");
    assert_eq!(sum[0].i64(), Some(3));
    assert_eq!(sum[1].i64(), Some(0));

    // Carry
    // 0xffff_ffff_ffff_ffff + 1 = 0x1_0000_0000_0000_0000
    add.call(
        &mut store,
        &[Val::I64(-1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut sum,
    )
    .expect("call to add-int failed");
    assert_eq!(sum[0].i64(), Some(0));
    assert_eq!(sum[1].i64(), Some(1));

    // Overflow in signed 64-bit, but fine in 128-bit
    // 0x7fff_ffff_ffff_ffff + 0x7fff_ffff_ffff_ffff = 0xffff_ffff_ffff_fffe
    add.call(
        &mut store,
        &[
            Val::I64(0x7fff_ffff_ffff_ffff),
            Val::I64(0),
            Val::I64(0x7fff_ffff_ffff_ffff),
            Val::I64(0),
        ],
        &mut sum,
    )
    .expect("call to add-int failed");
    assert_eq!(sum[0].i64(), Some(-2));
    assert_eq!(sum[1].i64(), Some(0));

    // Overflow
    // 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff + 1 = Overflow
    add.call(
        &mut store,
        &[
            Val::I64(-1),
            Val::I64(0x7fff_ffff_ffff_ffff),
            Val::I64(1),
            Val::I64(0),
        ],
        &mut sum,
    )
    .expect_err("expected overflow");

    // Overflow
    // 1 + 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff = Overflow
    add.call(
        &mut store,
        &[
            Val::I64(1),
            Val::I64(0),
            Val::I64(-1),
            Val::I64(0x7fff_ffff_ffff_ffff),
        ],
        &mut sum,
    )
    .expect_err("expected overflow");

    // Overflow
    // 0x8000_0000_0000_0000_0000_0000_0000_0000 + -1 = Overflow
    add.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(-9223372036854775808),
            Val::I64(-1),
            Val::I64(-1),
        ],
        &mut sum,
    )
    .expect_err("expected overflow");
}

#[test]
fn test_sub_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let sub = instance.get_func(&mut store, "sub-uint").unwrap();
    let mut sum = [Val::I64(0), Val::I64(0)];

    // 0 - 0 = 0
    sub.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-uint failed");
    assert_eq!(sum[0].i64(), Some(0));
    assert_eq!(sum[1].i64(), Some(0));

    // 3 - 2 = 1
    sub.call(
        &mut store,
        &[Val::I64(3), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-uint failed");
    assert_eq!(sum[0].i64(), Some(1));
    assert_eq!(sum[1].i64(), Some(0));

    // Borrow
    // 0x1_0000_0000_0000_0000 - 1 = 0xffff_ffff_ffff_ffff
    sub.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-uint failed");
    assert_eq!(sum[0].i64(), Some(-1));
    assert_eq!(sum[1].i64(), Some(0));

    // Signed underflow, but fine for unsigned
    // 0x8000_0000_0000_0000_0000_0000_0000_0000 - 1 = 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff
    sub.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(-9223372036854775808),
            Val::I64(1),
            Val::I64(0),
        ],
        &mut sum,
    )
    .expect("call to sub-uint failed");
    assert_eq!(sum[0].i64(), Some(-1));
    assert_eq!(sum[1].i64(), Some(0x7fff_ffff_ffff_ffff));

    // Underflow
    // 1 - 2 = Underflow
    sub.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut sum,
    )
    .expect_err("expected underflow");
}

#[test]
fn test_sub_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let sub = instance.get_func(&mut store, "sub-int").unwrap();
    let mut sum = [Val::I64(0), Val::I64(0)];

    // 0 - 0 = 0
    sub.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-int failed");
    assert_eq!(sum[0].i64(), Some(0));
    assert_eq!(sum[1].i64(), Some(0));

    // 3 - 2 = 1
    sub.call(
        &mut store,
        &[Val::I64(3), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-int failed");
    assert_eq!(sum[0].i64(), Some(1));
    assert_eq!(sum[1].i64(), Some(0));

    // 1 - 2 = -1
    sub.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-int failed");
    assert_eq!(sum[0].i64(), Some(-1));
    assert_eq!(sum[1].i64(), Some(-1));

    // Borrow
    // 0x1_0000_0000_0000_0000 - 1 = 0xffff_ffff_ffff_ffff
    sub.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut sum,
    )
    .expect("call to sub-int failed");
    assert_eq!(sum[0].i64(), Some(-1));
    assert_eq!(sum[1].i64(), Some(0));

    // Underflow
    // 0x8000_0000_0000_0000_0000_0000_0000_0000 - 1 = Underflow
    sub.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(-9223372036854775808),
            Val::I64(1),
            Val::I64(0),
        ],
        &mut sum,
    )
    .expect_err("expected underflow");
}

#[test]
fn test_mul_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let mul = instance.get_func(&mut store, "mul-uint").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // 0 * 0 = 0
    mul.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to mul-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0 * 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210 = 0
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0),
            Val::I64(-81985529216486896),
            Val::I64(0x0123_4567_89ab_cdef),
        ],
        &mut result,
    )
    .expect("call to mul-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210 * 0 = 0
    mul.call(
        &mut store,
        &[
            Val::I64(-81985529216486896),
            Val::I64(0x0123_4567_89ab_cdef),
            Val::I64(0),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect("call to mul-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 1 * 2 = 2
    mul.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect("call to mul-uint failed");
    assert_eq!(result[0].i64(), Some(2));
    assert_eq!(result[1].i64(), Some(0));

    // 0xffff_ffff_ffff_ffff * 0xffff_ffff_ffff_ffff = 0xffff_ffff_ffff_fffe_0000_0000_0000_0001
    mul.call(
        &mut store,
        &[Val::I64(-1), Val::I64(0), Val::I64(-1), Val::I64(0)],
        &mut result,
    )
    .expect("call to mul-uint failed");
    assert_eq!(result[0].i64(), Some(1));
    assert_eq!(result[1].i64(), Some(-2));

    // Overflow
    // 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 2 = Overflow
    mul.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow (a2b2)
    // 0x1_0000_0000_0000_0000 * 0x1_0000_0000_0000_0000 = Overflow
    mul.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow (a3b1)
    // 0x1_0000_0000_0000_0000_0000_0000 * 0x1_0000_0000 = Overflow
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0x1_0000_0000),
            Val::I64(0x1_0000_0000),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow (a1b3)
    // 0x1_0000_0000 * 0x1_0000_0000_0000_0000_0000_0000 = Overflow
    mul.call(
        &mut store,
        &[
            Val::I64(0x1_0000_0000),
            Val::I64(0),
            Val::I64(0),
            Val::I64(0x1_0000_0000),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow (a3b2)
    // 0x1_0000_0000_0000_0000_0000_0000 * 0x1_0000_0000_0000_0000 = Overflow
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0x1_0000_0000),
            Val::I64(0),
            Val::I64(1),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow (a2b3)
    // 0x1_0000_0000_0000_0000 * 0x1_0000_0000_0000_0000_0000_0000 = Overflow
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(1),
            Val::I64(0),
            Val::I64(0x1_0000_0000),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow (a3b3)
    // 0x1_0000_0000_0000_0000_0000_0000 * 0x1_0000_0000_0000_0000_0000_0000 = Overflow
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0x1_0000_0000),
            Val::I64(0),
            Val::I64(0x1_0000_0000),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 2 = 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_fffe
    mul.call(
        &mut store,
        &[
            Val::I64(-1),
            Val::I64(9223372036854775807),
            Val::I64(2),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect("call to mul-uint failed");
    assert_eq!(result[0].i64(), Some(-2));
    assert_eq!(result[1].i64(), Some(-1));

    // 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 3 = Overflow
    mul.call(
        &mut store,
        &[
            Val::I64(-1),
            Val::I64(9223372036854775807),
            Val::I64(3),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect_err("expected overflow");
}

#[test]
fn test_mul_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let mul = instance.get_func(&mut store, "mul-int").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // 0 * 0 = 0
    mul.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0 * 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210 = 0
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0),
            Val::I64(-81985529216486896),
            Val::I64(0x0123_4567_89ab_cdef),
        ],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210 * 0 = 0
    mul.call(
        &mut store,
        &[
            Val::I64(-81985529216486896),
            Val::I64(0x0123_4567_89ab_cdef),
            Val::I64(0),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 1 * 2 = 2
    mul.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(2));
    assert_eq!(result[1].i64(), Some(0));

    // 0xffff_ffff_ffff_ffff * 0xffff_ffff_ffff_ffff = 0xffff_ffff_ffff_fffe_0000_0000_0000_0001
    mul.call(
        &mut store,
        &[Val::I64(-1), Val::I64(0), Val::I64(-1), Val::I64(0)],
        &mut result,
    )
    .expect_err("expected overflow");

    // Overflow on unsigned multiplication
    // 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 2 = -2
    mul.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(-2));
    assert_eq!(result[1].i64(), Some(-1));

    // cannot take the absolute value of i128::MIN but should be able to multiply by 1
    mul.call(
        &mut store,
        &[
            Val::I64(1),
            Val::I64(0),
            Val::I64(0),
            Val::I64(0x8000000000000000u64 as i64),
        ],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0x8000000000000000u64 as i64));

    // cannot take the absolute value of i128::MIN but should be able to multiply by 1 (reverse)
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0x8000000000000000u64 as i64),
            Val::I64(1),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect("call to mul-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0x8000000000000000u64 as i64));

    // i128::MIN * 2 overflows
    mul.call(
        &mut store,
        &[
            Val::I64(2),
            Val::I64(0),
            Val::I64(0),
            Val::I64(0x8000000000000000u64 as i64),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // i128::MIN * -1 overflows
    mul.call(
        &mut store,
        &[
            Val::I64(-1),
            Val::I64(-1),
            Val::I64(0),
            Val::I64(0x8000000000000000u64 as i64),
        ],
        &mut result,
    )
    .expect_err("expected overflow");

    // -1 * i128::MIN overflows
    mul.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(0x8000000000000000u64 as i64),
            Val::I64(-1),
            Val::I64(-1),
        ],
        &mut result,
    )
    .expect_err("expected overflow");
}

#[test]
fn test_div_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let div = instance.get_func(&mut store, "div-uint").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // 4 / 2 = 2
    div.call(
        &mut store,
        &[Val::I64(4), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect("call to div-uint failed");
    assert_eq!(result[0].i64(), Some(2));
    assert_eq!(result[1].i64(), Some(0));

    // 7 / 4 = 1
    div.call(
        &mut store,
        &[Val::I64(7), Val::I64(0), Val::I64(4), Val::I64(0)],
        &mut result,
    )
    .expect("call to div-uint failed");
    assert_eq!(result[0].i64(), Some(1));
    assert_eq!(result[1].i64(), Some(0));

    // 123 / 456 = 0
    div.call(
        &mut store,
        &[Val::I64(123), Val::I64(0), Val::I64(456), Val::I64(0)],
        &mut result,
    )
    .expect("call to div-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0 / 0x123_0000_0000_0000_0456 = 0
    div.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(0x456), Val::I64(0x123)],
        &mut result,
    )
    .expect("call to div-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0x123_0000_0000_0000_0456 / 0 = DivideByZero
    div.call(
        &mut store,
        &[Val::I64(0x456), Val::I64(0x123), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect_err("expected divide by zero");

    // 0x123_0000_0000_0000_0456 / 22 = 0xd_3a2e_8ba2_e8ba_2ebe
    div.call(
        &mut store,
        &[Val::I64(0x456), Val::I64(0x123), Val::I64(22), Val::I64(0)],
        &mut result,
    )
    .expect("call to div-uint failed");
    assert_eq!(result[0].i64(), Some(0x3a2e_8ba2_e8ba_2ebe));
    assert_eq!(result[1].i64(), Some(0xd));
}

#[test]
fn test_div_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let div = instance.get_func(&mut store, "div-int").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // 4 / 2 = 2
    div.call(
        &mut store,
        &[Val::I64(4), Val::I64(0), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect("call to div-int failed");
    assert_eq!(result[0].i64(), Some(2));
    assert_eq!(result[1].i64(), Some(0));

    // -4 / 2 = -2
    div.call(
        &mut store,
        &[Val::I64(-4), Val::I64(-1), Val::I64(2), Val::I64(0)],
        &mut result,
    )
    .expect("call to div-int failed");
    assert_eq!(result[0].i64(), Some(-2));
    assert_eq!(result[1].i64(), Some(-1));

    // 4 / -2 = -2
    div.call(
        &mut store,
        &[Val::I64(4), Val::I64(0), Val::I64(-2), Val::I64(-1)],
        &mut result,
    )
    .expect("call to div-int failed");
    assert_eq!(result[0].i64(), Some(-2));
    assert_eq!(result[1].i64(), Some(-1));

    // -4 / -2 = 2
    div.call(
        &mut store,
        &[Val::I64(-4), Val::I64(-1), Val::I64(-2), Val::I64(-1)],
        &mut result,
    )
    .expect("call to div-int failed");
    assert_eq!(result[0].i64(), Some(2));
    assert_eq!(result[1].i64(), Some(0));

    // 0x8000_0000_0000_0000_0000_0000_0000_0000 / -2 = 0xc000_0000_0000_0000_0000_0000_0000_0000
    div.call(
        &mut store,
        &[
            Val::I64(0),
            Val::I64(-9223372036854775808),
            Val::I64(2),
            Val::I64(0),
        ],
        &mut result,
    )
    .expect("call to div-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(-4611686018427387904i64));
}

#[test]
fn test_mod_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let modulo = instance.get_func(&mut store, "mod-uint").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // 4 % 2 = 0
    modulo
        .call(
            &mut store,
            &[Val::I64(4), Val::I64(0), Val::I64(2), Val::I64(0)],
            &mut result,
        )
        .expect("call to mod-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 7 % 4 = 3
    modulo
        .call(
            &mut store,
            &[Val::I64(7), Val::I64(0), Val::I64(4), Val::I64(0)],
            &mut result,
        )
        .expect("call to mod-uint failed");
    assert_eq!(result[0].i64(), Some(3));
    assert_eq!(result[1].i64(), Some(0));

    // 123 % 456 = 123
    modulo
        .call(
            &mut store,
            &[Val::I64(123), Val::I64(0), Val::I64(456), Val::I64(0)],
            &mut result,
        )
        .expect("call to mod-uint failed");
    assert_eq!(result[0].i64(), Some(123));
    assert_eq!(result[1].i64(), Some(0));

    // 0 % 0x123_0000_0000_0000_0456 = 0
    modulo
        .call(
            &mut store,
            &[Val::I64(0), Val::I64(0), Val::I64(0x456), Val::I64(0x123)],
            &mut result,
        )
        .expect("call to mod-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // 0x123_0000_0000_0000_0456 % 0 = DivideByZero
    modulo
        .call(
            &mut store,
            &[Val::I64(0x456), Val::I64(0x123), Val::I64(0), Val::I64(0)],
            &mut result,
        )
        .expect_err("expected divide by zero");

    // 0x123_0000_0000_0000_0456 % 22 = 2
    modulo
        .call(
            &mut store,
            &[Val::I64(0x456), Val::I64(0x123), Val::I64(22), Val::I64(0)],
            &mut result,
        )
        .expect("call to mod-uint failed");
    assert_eq!(result[0].i64(), Some(2));
    assert_eq!(result[1].i64(), Some(0));
}

#[test]
fn test_mod_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let modulo = instance.get_func(&mut store, "mod-int").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // 7 % 4 = 3
    modulo
        .call(
            &mut store,
            &[Val::I64(7), Val::I64(0), Val::I64(4), Val::I64(0)],
            &mut result,
        )
        .expect("call to mod-int failed");
    assert_eq!(result[0].i64(), Some(3));
    assert_eq!(result[1].i64(), Some(0));

    // -7 / 4 = -3
    modulo
        .call(
            &mut store,
            &[Val::I64(-7), Val::I64(-1), Val::I64(4), Val::I64(0)],
            &mut result,
        )
        .expect("call to mod-int failed");
    assert_eq!(result[0].i64(), Some(-3));
    assert_eq!(result[1].i64(), Some(-1));

    // 7 / -4 = 3
    modulo
        .call(
            &mut store,
            &[Val::I64(7), Val::I64(0), Val::I64(-4), Val::I64(-1)],
            &mut result,
        )
        .expect("call to mod-int failed");
    assert_eq!(result[0].i64(), Some(3));
    assert_eq!(result[1].i64(), Some(0));

    // -7 / -4 = -3
    modulo
        .call(
            &mut store,
            &[Val::I64(-7), Val::I64(-1), Val::I64(-4), Val::I64(-1)],
            &mut result,
        )
        .expect("call to mod-int failed");
    assert_eq!(result[0].i64(), Some(-3));
    assert_eq!(result[1].i64(), Some(-1));

    // 0x123_0000_0000_0000_0456 % 0 = DivideByZero
    modulo
        .call(
            &mut store,
            &[Val::I64(0x456), Val::I64(0x123), Val::I64(0), Val::I64(0)],
            &mut result,
        )
        .expect_err("expected divide by zero");
}

#[test]
fn test_lt_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let lt = instance.get_func(&mut store, "lt-uint").unwrap();
    let mut result = [Val::I32(0)];

    // 0 < 1 is true
    lt.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 < 0 is false
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 < 1 is false
    lt.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 < 0x1_0000_0000_0000_0000 is true
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 < 0x1_0000_0000_0000_0001 is true
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0000 < 1 is false
    lt.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0001 < 1 is false
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0000 < 0x1_0000_0000_0000_0001 is true
    lt.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0001 < 0x1_0000_0000_0000_0000 is false
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0001 < 0x1_0000_0000_0000_0001 is false
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // u128::MAX (-1 if signed) < 1 is false
    lt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 < u128::MAX (-1 if signed) is true
    lt.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));
}

#[test]
fn test_gt_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let gt = instance.get_func(&mut store, "gt-uint").unwrap();
    let mut result = [Val::I32(0)];

    // 0 > 1 is false
    gt.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 > 0 is true
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 > 1 is false
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 > 0x1_0000_0000_0000_0000 is false
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 > 0x1_0000_0000_0000_0001 is false
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0000 > 1 is true
    gt.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0001 > 1 is true
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0000 > 0x1_0000_0000_0000_0001 is false
    gt.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0001 > 0x1_0000_0000_0000_0000 is true
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0001 > 0x1_0000_0000_0000_0001 is false
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to gt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // u128::MAX (-1 if signed) > 1 is true
    gt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 > u128::MAX (-1 if signed) is false
    gt.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));
}

#[test]
fn test_le_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let le = instance.get_func(&mut store, "le-uint").unwrap();
    let mut result = [Val::I32(0)];

    // 0 <= 1 is true
    le.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 <= 0 is false
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 <= 1 is true
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 <= 0x1_0000_0000_0000_0000 is true
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 <= 0x1_0000_0000_0000_0001 is true
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0000 <= 1 is false
    le.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0001 <= 1 is false
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0000 <= 0x1_0000_0000_0000_0001 is true
    le.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0001 <= 0x1_0000_0000_0000_0000 is false
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0001 <= 0x1_0000_0000_0000_0001 is true
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to le-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // u128::MAX (-1 if signed) <= 1 is false
    le.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 <= u128::MAX (-1 if signed) is true
    le.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));
}

#[test]
fn test_ge_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let ge = instance.get_func(&mut store, "ge-uint").unwrap();
    let mut result = [Val::I32(0)];

    // 0 >= 1 is false
    ge.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 >= 0 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 >= 1 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 >= 0x1_0000_0000_0000_0000 is false
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 >= 0x1_0000_0000_0000_0001 is false
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0000 >= 1 is true
    ge.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0001 >= 1 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0000 >= 0x1_0000_0000_0000_0001 is false
    ge.call(
        &mut store,
        &[Val::I64(0), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(0));

    // 0x1_0000_0000_0000_0001 >= 0x1_0000_0000_0000_0000 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(0), Val::I64(1)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 0x1_0000_0000_0000_0001 >= 0x1_0000_0000_0000_0001 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(1), Val::I64(1), Val::I64(1)],
        &mut result,
    )
    .expect("call to ge-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // u128::MAX (-1 if signed) >= 1 is true
    ge.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 >= u128::MAX (-1 if signed) is false
    ge.call(
        &mut store,
        &[Val::I64(0), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-uint failed");
    assert_eq!(result[0].i32(), Some(0));
}

#[test]
fn test_lt_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let lt = instance.get_func(&mut store, "lt-int").unwrap();
    let mut result = [Val::I32(0)];

    // 1 < 1 is false
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 < -1 is false
    lt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 < 1 is true
    lt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 < -1 is false
    lt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 < 0 is true
    lt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -2 < -1 is true
    lt.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -2 < -3 is false
    lt.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-3), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // I128::MIN < -1 is true
    lt.call(
        &mut store,
        &[Val::I64(0), Val::I64(i64::MIN), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 < I128::MIN is false
    lt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(i64::MIN)],
        &mut result,
    )
    .expect("call to lt-int failed");
    assert_eq!(result[0].i32(), Some(0));
}

#[test]
fn test_gt_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let gt = instance.get_func(&mut store, "gt-int").unwrap();
    let mut result = [Val::I32(0)];

    // 1 > 1 is false
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 > -1 is false
    gt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 > 1 is false
    gt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 > -1 is true
    gt.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 > 0 is false
    gt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -2 > -1 is false
    gt.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -2 > -3 is true
    gt.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-3), Val::I64(-1)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // I128::MIN > -1 is false
    gt.call(
        &mut store,
        &[Val::I64(0), Val::I64(i64::MIN), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 > I128::MIN is true
    gt.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(i64::MIN)],
        &mut result,
    )
    .expect("call to gt-int failed");
    assert_eq!(result[0].i32(), Some(1));
}

#[test]
fn test_le_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let le = instance.get_func(&mut store, "le-int").unwrap();
    let mut result = [Val::I32(0)];

    // 1 <= 1 is true
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 <= -1 is true
    le.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 <= 1 is true
    le.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // 1 <= -1 is false
    le.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 <= 0 is true
    le.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -2 <= -1 is true
    le.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -2 <= -3 is false
    le.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-3), Val::I64(-1)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // I128::MIN <= -1 is true
    le.call(
        &mut store,
        &[Val::I64(0), Val::I64(i64::MIN), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 <= I128::MIN is false
    le.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(i64::MIN)],
        &mut result,
    )
    .expect("call to le-int failed");
    assert_eq!(result[0].i32(), Some(0));
}

#[test]
fn test_ge_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let ge = instance.get_func(&mut store, "ge-int").unwrap();
    let mut result = [Val::I32(0)];

    // 1 >= 1 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 >= -1 is true
    ge.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 >= 1 is false
    ge.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(1), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // 1 >= -1 is true
    ge.call(
        &mut store,
        &[Val::I64(1), Val::I64(0), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // -1 >= 0 is false
    ge.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(0)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -2 >= -1 is false
    ge.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -2 >= -3 is true
    ge.call(
        &mut store,
        &[Val::I64(-2), Val::I64(-1), Val::I64(-3), Val::I64(-1)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(1));

    // I128::MIN >= -1 is false
    ge.call(
        &mut store,
        &[Val::I64(0), Val::I64(i64::MIN), Val::I64(-1), Val::I64(-1)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(0));

    // -1 >= I128::MIN is true
    ge.call(
        &mut store,
        &[Val::I64(-1), Val::I64(-1), Val::I64(0), Val::I64(i64::MIN)],
        &mut result,
    )
    .expect("call to ge-int failed");
    assert_eq!(result[0].i32(), Some(1));
}

#[test]
fn test_log2_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let log2 = instance.get_func(&mut store, "log2-uint").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // log2(0) is an error
    log2.call(&mut store, &[Val::I64(0), Val::I64(0)], &mut result)
        .expect_err("expected log of 0 error");

    // log2(u128::MAX) is not an error (-1 if signed)
    log2.call(&mut store, &[Val::I64(-1), Val::I64(-1)], &mut result)
        .expect("call to log2-uint failed");
    assert_eq!(result[0].i64(), Some(127));
    assert_eq!(result[1].i64(), Some(0));

    // log2(u64::MAX) is not an error
    log2.call(&mut store, &[Val::I64(-1), Val::I64(0)], &mut result)
        .expect("call to log2-uint failed");
    assert_eq!(result[0].i64(), Some(63));
    assert_eq!(result[1].i64(), Some(0));

    // log2(u128::MAX-u64::MAX) is not an error
    log2.call(&mut store, &[Val::I64(0), Val::I64(-1)], &mut result)
        .expect("call to log2-uint failed");
    assert_eq!(result[0].i64(), Some(127));
    assert_eq!(result[1].i64(), Some(0));
}

#[test]
fn test_log2_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let log2 = instance.get_func(&mut store, "log2-int").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // log2(0) is an error
    log2.call(&mut store, &[Val::I64(0), Val::I64(0)], &mut result)
        .expect_err("expected log of 0 error");

    // log2(-1) is an error
    log2.call(&mut store, &[Val::I64(-1), Val::I64(-1)], &mut result)
        .expect_err("expected log of negative number error");

    // log2(u64::MAX) is not an error
    log2.call(&mut store, &[Val::I64(-1), Val::I64(0)], &mut result)
        .expect("call to log2-int failed");
    assert_eq!(result[0].i64(), Some(63));
    assert_eq!(result[1].i64(), Some(0));

    // log2(u128::MAX-u64::MAX) is an error
    log2.call(&mut store, &[Val::I64(0), Val::I64(-1)], &mut result)
        .expect_err("expected log of negative number error");
}

#[test]
fn test_sqrti_uint() {
    let (instance, mut store) = load_stdlib().unwrap();
    let sqrti = instance.get_func(&mut store, "sqrti-uint").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // sqrti(0) = 0
    sqrti
        .call(&mut store, &[Val::I64(0), Val::I64(0)], &mut result)
        .expect("call to sqrti-uint failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(1) = 1
    sqrti
        .call(&mut store, &[Val::I64(1), Val::I64(0)], &mut result)
        .expect("call to sqrti-uint failed");
    assert_eq!(result[0].i64(), Some(1));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(0xffff_ffff_ffff_ffff) = 0xffff_ffff
    sqrti
        .call(&mut store, &[Val::I64(-1), Val::I64(0)], &mut result)
        .expect("call to sqrti-uint failed");
    assert_eq!(result[0].i64(), Some(0xffff_ffff));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(0x1_0000_0000_0000_0000) = 0x1_0000_0000
    sqrti
        .call(&mut store, &[Val::I64(0), Val::I64(1)], &mut result)
        .expect("call to sqrti-uint failed");
    assert_eq!(result[0].i64(), Some(0x1_0000_0000));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(u128::MAX)  = 0xffff_ffff_ffff_ffff
    sqrti
        .call(&mut store, &[Val::I64(-1), Val::I64(-1)], &mut result)
        .expect("call to sqrti-uint failed");
    assert_eq!(result[0].i64(), Some(-1));
    assert_eq!(result[1].i64(), Some(0));
}

#[test]
fn test_sqrti_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let sqrti = instance.get_func(&mut store, "sqrti-int").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // sqrti(0) = 0
    sqrti
        .call(&mut store, &[Val::I64(0), Val::I64(0)], &mut result)
        .expect("call to sqrti-int failed");
    assert_eq!(result[0].i64(), Some(0));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(1) = 1
    sqrti
        .call(&mut store, &[Val::I64(1), Val::I64(0)], &mut result)
        .expect("call to sqrti-int failed");
    assert_eq!(result[0].i64(), Some(1));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(0xffff_ffff_ffff_ffff) = 0xffff_ffff
    sqrti
        .call(&mut store, &[Val::I64(-1), Val::I64(0)], &mut result)
        .expect("call to sqrti-int failed");
    assert_eq!(result[0].i64(), Some(0xffff_ffff));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(0x1_0000_0000_0000_0000) = 0x1_0000_0000
    sqrti
        .call(&mut store, &[Val::I64(0), Val::I64(1)], &mut result)
        .expect("call to sqrti-int failed");
    assert_eq!(result[0].i64(), Some(0x1_0000_0000));
    assert_eq!(result[1].i64(), Some(0));

    // sqrti(-1) is error
    sqrti
        .call(&mut store, &[Val::I64(-1), Val::I64(-1)], &mut result)
        .expect_err("expected sqrti of negative integer");
}

#[test]
fn bit_not_int() {
    let (instance, mut store) = load_stdlib().unwrap();
    let bitnot = instance.get_func(&mut store, "bit-not-int").unwrap();
    let mut result = [Val::I64(0), Val::I64(0)];

    // bit-not(3) = -4
    bitnot
        .call(&mut store, &[Val::I64(3), Val::I64(0)], &mut result)
        .expect("call to bit-not failed");
    assert_eq!(result[0].i64(), Some(-4));
    assert_eq!(result[1].i64(), Some(-1));
}
