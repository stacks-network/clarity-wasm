use crate::utils;

#[test]
fn prop_add_uint() {
    utils::test_export_two_unsigned_args_checked("add-uint", |a: u128, b: u128| a.checked_add(b))
}

#[test]
fn prop_add_int() {
    utils::test_export_two_signed_args_checked("add-int", |a: i128, b: i128| a.checked_add(b))
}

#[test]
fn prop_sub_uint() {
    utils::test_export_two_unsigned_args_checked("sub-uint", |a: u128, b: u128| a.checked_sub(b))
}

#[test]
fn prop_sub_int() {
    utils::test_export_two_signed_args_checked("sub-int", |a: i128, b: i128| a.checked_sub(b))
}

#[test]
fn prop_mul_uint() {
    utils::test_export_two_unsigned_args_checked("mul-uint", |a: u128, b: u128| a.checked_mul(b))
}

#[test]
fn prop_mul_int() {
    utils::test_export_two_signed_args_checked("mul-int", |a: i128, b: i128| a.checked_mul(b))
}

#[test]
fn prop_div_uint() {
    utils::test_export_two_unsigned_args_checked("div-uint", |a: u128, b: u128| a.checked_div(b))
}

#[test]
fn prop_div_int() {
    utils::test_export_two_signed_args_checked("div-int", |a: i128, b: i128| a.checked_div(b))
}

#[test]
fn prop_mod_uint() {
    utils::test_export_two_unsigned_args_checked("mod-uint", |a: u128, b: u128| a.checked_rem(b))
}

#[test]
fn prop_mod_int() {
    utils::test_export_two_signed_args_checked("mod-int", |a: i128, b: i128| a.checked_rem(b))
}

#[test]
fn prop_lt_uint() {
    utils::test_export_two_unsigned_args("lt-uint", |a: u128, b: u128| a < b)
}

#[test]
fn prop_lt_int() {
    utils::test_export_two_signed_args("lt-int", |a: i128, b: i128| a < b);
}

#[test]
fn prop_gt_uint() {
    utils::test_export_two_unsigned_args("gt-uint", |a: u128, b: u128| a > b);
}

#[test]
fn prop_gt_int() {
    utils::test_export_two_signed_args("gt-int", |a: i128, b: i128| a > b);
}

#[test]
fn prop_le_uint() {
    utils::test_export_two_unsigned_args("le-uint", |a: u128, b: u128| a <= b);
}

#[test]
fn prop_le_int() {
    utils::test_export_two_signed_args("le-int", |a: i128, b: i128| a <= b);
}

#[test]
fn prop_ge_uint() {
    utils::test_export_two_unsigned_args("ge-uint", |a: u128, b: u128| a >= b);
}

#[test]
fn prop_ge_int() {
    utils::test_export_two_signed_args("ge-int", |a: i128, b: i128| a >= b);
}

#[test]
fn prop_log2_uint() {
    utils::test_export_one_unsigned_arg_checked("log2-uint", |a: u128| {
        a.checked_ilog2().map(|u| u as u128)
    })
}

#[test]
fn prop_log2_int() {
    utils::test_export_one_signed_arg_checked("log2-int", |a: i128| {
        a.checked_ilog2().map(|u| u as i128)
    })
}

#[test]
fn prop_sqrti_uint() {
    utils::test_export_one_unsigned_arg("sqrti-uint", |a: u128| num_integer::Roots::sqrt(&a))
}

#[test]
fn prop_sqrti_int() {
    utils::test_export_one_signed_arg_checked("sqrti-int", |a: i128| {
        (a >= 0).then(|| num_integer::Roots::sqrt(&a))
    })
}

#[test]
fn prop_bit_and_uint() {
    utils::test_export_two_unsigned_args("bit-and-uint", |a: u128, b: u128| a & b)
}

#[test]
fn prop_bit_and_int() {
    utils::test_export_two_signed_args("bit-and-int", |a: i128, b: i128| a & b)
}

#[test]
fn prop_bit_or_uint() {
    utils::test_export_two_unsigned_args("bit-or-uint", |a: u128, b: u128| a | b)
}

#[test]
fn prop_bit_or_int() {
    utils::test_export_two_signed_args("bit-or-int", |a: i128, b: i128| a | b)
}

#[test]
fn prop_bit_not_uint() {
    utils::test_export_one_unsigned_arg("bit-not-uint", |a: u128| !a)
}

#[test]
fn prop_bit_not_int() {
    utils::test_export_one_signed_arg("bit-not-int", |a: i128| !a)
}

#[test]
fn prop_bit_xor_uint() {
    utils::test_export_two_unsigned_args("bit-xor-uint", |a: u128, b: u128| a ^ b)
}

#[test]
fn prop_bit_xor_int() {
    utils::test_export_two_signed_args("bit-xor-int", |a: i128, b: i128| a ^ b)
}

#[test]
fn prop_bit_shift_left_uint() {
    utils::test_export_two_unsigned_args("bit-shift-left-uint", |a: u128, b: u128| {
        a.wrapping_shl((b % 128) as u32)
    })
}

#[test]
fn prop_bit_shift_left_int() {
    // NOTE that the two arguments differ in type
    utils::test_export_two_signed_args("bit-shift-left-int", |a: i128, b: u128| {
        a.wrapping_shl((b % 128) as u32)
    })
}

#[test]
fn prop_bit_shift_right_uint() {
    utils::test_export_two_unsigned_args("bit-shift-right-uint", |a: u128, b: u128| {
        a.wrapping_shr((b % 128) as u32)
    })
}

#[test]
fn prop_bit_shift_right_int() {
    // NOTE that the two arguments differ in type
    utils::test_export_two_signed_args("bit-shift-right-int", |a: i128, b: u128| {
        a.wrapping_shr((b % 128) as u32)
    })
}
