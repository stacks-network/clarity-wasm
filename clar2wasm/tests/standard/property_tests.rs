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
