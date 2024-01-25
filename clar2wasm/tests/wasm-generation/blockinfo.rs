use clar2wasm::tools::{crosscheck_compare_only, TestEnvironment};
use proptest::proptest;

use crate::{block_range, uint};

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
    #[test]
    fn crossprop_blockinfo_within_controlled_range(block_height in block_range(STACKS_BLOCK_HEIGHT_LIMIT)) {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(STACKS_BLOCK_HEIGHT_LIMIT);

        for info in &BLOCK_INFO {
            crosscheck_compare_only(
                &format!("(get-block-info? {info} {block_height})")
            )
        }
    }
}

proptest! {
    #[test]
    fn crossprop_blockinfo(block_height in uint()) {
        for info in &BLOCK_INFO {
            println!("Here!");
            crosscheck_compare_only(
                &format!("(get-block-info? {info} {block_height})")
            )
        }
    }
}

proptest! {
    #[test]
    fn crossprop_blockinfo_burnchain_within_controlled_range(block_height in block_range(BURN_BLOCK_HEIGHT_LIMIT)) {
        let mut env = TestEnvironment::default();
        env.advance_chain_tip(STACKS_BLOCK_HEIGHT_LIMIT);

        for info in &BURN_BLOCK_INFO {
            crosscheck_compare_only(
                &format!("(get-burn-block-info? {info} {block_height})")
            )
        }
    }
}

proptest! {
    #[test]
    fn crossprop_blockinfo_burnchain(block_height in uint()) {
        for info in &BURN_BLOCK_INFO {
            crosscheck_compare_only(
                &format!("(get-burn-block-info? {info} {block_height})")
            )
        }
    }
}
