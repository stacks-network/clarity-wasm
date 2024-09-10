use clar2wasm::tools::{crosscheck, crosscheck_compare_only_advancing_tip};
use clarity::vm::types::SequenceData;
use clarity::vm::Value;
use proptest::prelude::Strategy;
use proptest::proptest;

use crate::{buffer, uint, PropValue};

const BLOCK_INFO_V1: [&str; 5] = [
    "burnchain-header-hash",
    "id-header-hash",
    "header-hash",
    "miner-address",
    "time",
];
const BLOCK_INFO_V2: [&str; 3] = ["block-reward", "miner-spend-total", "miner-spend-winner"];

const STACKS_BLOCK_HEIGHT_LIMIT: u32 = 100;

#[cfg(feature = "test-clarity-v1")]
mod clarity_v1 {
    use super::*;
    use crate::runtime_config;

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crossprop_blockinfo_within_controlled_range(block_height in 1..=STACKS_BLOCK_HEIGHT_LIMIT, tip in 1..=80u32) {
            for info in &BLOCK_INFO_V1 {
                crosscheck_compare_only_advancing_tip(&format!("(get-block-info? {info} u{block_height})"), tip)
            }
        }
    }
}

#[cfg(feature = "test-clarity-v2")]
mod clarity_v2 {
    use super::*;
    use crate::runtime_config;

    proptest! {
        #![proptest_config(runtime_config())]

        #[test]
        fn crossprop_blockinfo_within_controlled_range(block_height in 1..=STACKS_BLOCK_HEIGHT_LIMIT, tip in 1..=80u32) {
            for info in BLOCK_INFO_V1.iter().chain(BLOCK_INFO_V2.iter()) {
                crosscheck_compare_only_advancing_tip(&format!("(get-block-info? {info} u{block_height})"), tip)
            }
        }
    }
}

#[cfg(not(any(feature = "test-clarity-v1", feature = "test-clarity-v2")))]
mod clarity_v3 {
    use super::*;
    use crate::runtime_config;

    proptest! {
        #![proptest_config(runtime_config())]

        #[ignore = "see issue #428"]
        #[test]
        fn crossprop_blockinfo_within_controlled_range(block_height in 1..=STACKS_BLOCK_HEIGHT_LIMIT, tip in 1..=80u32) {
            for info in BLOCK_INFO_V1.iter().chain(BLOCK_INFO_V2.iter()) {
                crosscheck_compare_only_advancing_tip(&format!("(get-stacks-block-info? {info} u{block_height})"), tip)
            }
        }
    }
}

#[cfg(not(feature = "test-clarity-v1"))]
mod clarity_v2_v3 {
    use super::*;
    use crate::runtime_config;

    const BURN_BLOCK_INFO: [&str; 2] = ["header-hash", "pox-addrs"];
    const BURN_BLOCK_HEIGHT_LIMIT: u32 = 100;

    proptest! {
        #![proptest_config(runtime_config())]

        # [test]
        fn crossprop_blockinfo_burnchain_within_controlled_range(block_height in 1..=BURN_BLOCK_HEIGHT_LIMIT, tip in 1..=80u32) {
            for info in &BURN_BLOCK_INFO {
                crosscheck_compare_only_advancing_tip(
                    &format!("(get-burn-block-info? {info} u{block_height})"), tip
                )
            }
        }
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    # [test]
    fn crosscheck_at_block(
        operand in uint().prop_map(PropValue::from),
        buf in buffer(32)
    ) {
        let expected = operand.clone();

        crosscheck(
            &format!("(at-block {buf} (+ u0 {operand}))"),
            Ok(Some(expected.into()))
        )
    }

    # [test]
    fn crosscheck_at_block_inner(
        seq in (1u8..=20).prop_flat_map(|size| PropValue::any_sequence(size as usize)),
        buf in buffer(32)
    ) {
        let expected: u128 = extract_sequence(seq.clone()).len().try_into().unwrap();

        crosscheck(
            &format!("(begin (at-block {buf} (len {seq})))"),
            Ok(Some(Value::UInt(expected)))
        )
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    # [test]
    fn crosscheck_as_contract(
        operand in uint().prop_map(PropValue::from)
    ) {
        let expected = operand.clone();

        crosscheck(
            &format!("(as-contract (+ u0 {operand}))"),
            Ok(Some(expected.into()))
        )
    }

    # [test]
    fn crosscheck_as_contract_inner(
        seq in (1u8..=20).prop_flat_map(|size| PropValue::any_sequence(size as usize))
    ) {
        let expected: u128 = extract_sequence(seq.clone()).len().try_into().unwrap();

        crosscheck(
            &format!("(begin (as-contract (len {seq})))"),
            Ok(Some(Value::UInt(expected)))
        )
    }
}

fn extract_sequence(sequence: PropValue) -> SequenceData {
    match Value::from(sequence) {
        Value::Sequence(seq_data) => seq_data,
        _ => panic!("Should only call this function on the result of PropValue::any_sequence"),
    }
}
