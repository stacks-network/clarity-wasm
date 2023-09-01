// These test is here to make sure that the logical behaviour of the algorithms neccesary to
// deal with (i64, i64)-tuples in wasm are actually correct.

// Since rust is much easier to debug than the wat format, having these functions as a reference
// can be valuable

// It also shows that we only need to consider the unsigned case for left shifts, since converting
// the input to unsigned and the result back to signed always yields the correct result

use proptest::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PropIntTiny(u16);

impl PropIntTiny {
    const fn new(n: u16) -> Self {
        Self(n)
    }

    const fn signed(&self) -> i16 {
        self.0 as i16
    }

    const fn unsigned(&self) -> u16 {
        self.0
    }
}

prop_compose! {
    fn int16()(n in any::<u16>()) -> PropIntTiny {
        PropIntTiny::new(n)
    }
}

pub(crate) fn bit_shift_left_split_uint(a: u16, by: u32) -> u16 {
    let by = (by % 16) as u32;

    let lo = a as u8;
    let hi = (a >> 8) as u8;

    if by == 0 {
        a
    } else if by >= 8 {
        let new_high: u8 = lo << (by - 8);
        (new_high as u16) << 8
    } else {
        let overflow = lo >> 8 - by;

        let new_lo = lo << by;
        let new_hi = hi << by | overflow;

        (new_hi as u16) << 8 | new_lo as u16
    }
}

pub(crate) fn bit_shift_right_split_uint(a: u16, by: u32) -> u16 {
    let by = (by % 16) as u32;

    let lo = a as u8;
    let hi = (a >> 8) as u8;

    if by == 0 {
        a
    } else if by >= 8 {
        let new_lo: u8 = hi >> (by - 8);
        new_lo as u16
    } else {
        let overflow = hi << (8 - by);

        let new_hi = hi >> by;
        let new_lo = lo >> by | overflow;

        (new_hi as u16) << 8 | new_lo as u16
    }
}

pub(crate) fn bit_shift_right_split_int(a: i16, by: u32) -> i16 {
    let by = (by % 16) as u32;

    if by == 0 {
        return a;
    }

    let lo = a as u8;
    let hi = (a >> 8) as i8; // high part remains signed

    if by >= 8 {
        (hi >> (by - 8)) as i16
    } else {
        let overflow = (hi << (8 - by)) as u8;

        let new_hi = hi as i8 >> by;
        let new_lo = lo >> by | overflow;

        (new_hi as i16) << 8 | new_lo as i16
    }
}

#[test]
fn prop_shift_left_reference_uint() {
    proptest!(|(n in int16(), m in int16())| {
        let m = (m.unsigned() % 16) as u32;

        let rust_result = n.unsigned() << m;
        let reference_result = bit_shift_left_split_uint(n.unsigned(), m);

        prop_assert_eq!(rust_result, reference_result);
    })
}

#[test]
fn prop_shift_left_reference_int() {
    proptest!(|(n in int16(), m in int16())| {
        let m = (m.unsigned() % 16) as u32;

        let rust_result = n.signed() << m;
        let reference_result = bit_shift_left_split_uint(n.unsigned(), m) as i16;

        prop_assert_eq!(rust_result, reference_result);
    })
}

#[test]
fn prop_shift_right_reference_uint() {
    proptest!(|(n in int16(), m in int16())| {
        let m = (m.unsigned() % 16) as u32;

        let rust_result = n.unsigned() >> m;
        let reference_result = bit_shift_right_split_uint(n.unsigned(), m);

        prop_assert_eq!(rust_result, reference_result);
    })
}

#[test]
fn prop_shift_right_reference_int() {
    proptest!(|(n in int16(), m in int16())| {
        let m = (m.unsigned() % 16) as u32;

        let rust_result = n.signed() >> m;
        let reference_result = bit_shift_right_split_int(n.signed(), m);

        prop_assert_eq!(rust_result, reference_result);
    })
}
