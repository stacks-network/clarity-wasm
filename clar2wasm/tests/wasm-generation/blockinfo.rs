use clar2wasm::tools::{crosscheck_compare_only, crosscheck_compare_only_advancing_tip};
use proptest::proptest;

use crate::uint;

const BLOCK_INFO: [&str; 8] = [
    "burnchain-header-hash",
    "id-header-hash",
    "header-hash",
    "miner-address",
    "block-reward",
    "miner-spend-total",
    "miner-spend-winner",
    "time",
];
const BURN_BLOCK_INFO: [&str; 2] = ["header-hash", "pox-addrs"];
const STACKS_BLOCK_HEIGHT_LIMIT: u32 = 100;
const BURN_BLOCK_HEIGHT_LIMIT: u32 = 100;

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_blockinfo_within_controlled_range(block_height in 1..=STACKS_BLOCK_HEIGHT_LIMIT) {
        for info in &BLOCK_INFO {
            crosscheck_compare_only_advancing_tip(&format!("(get-block-info? {info} u{block_height})"), 80)
        }
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_blockinfo(block_height in uint()) {
        for info in &BLOCK_INFO {
            crosscheck_compare_only(
                &format!("(get-block-info? {info} {block_height})")
            )
        }
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    # [test]
    fn crossprop_blockinfo_burnchain_within_controlled_range(block_height in 1..=BURN_BLOCK_HEIGHT_LIMIT) {
        for info in &BURN_BLOCK_INFO {
            crosscheck_compare_only_advancing_tip(
                &format!("(get-burn-block-info? {info} u{block_height})"), 80
            )
        }
    }
}

proptest! {
    #![proptest_config(super::runtime_config())]

    #[test]
    fn crossprop_blockinfo_burnchain(block_height in uint()) {
        for info in &BURN_BLOCK_INFO {
            crosscheck_compare_only(
                &format!("(get-burn-block-info? {info} {block_height})")
            )
        }
    }
}
