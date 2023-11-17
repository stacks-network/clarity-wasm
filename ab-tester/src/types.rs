use color_eyre::Result;

use crate::{stacks, utils::*};

pub struct Snapshot {
    pub block_height: u32,
    pub burn_header_hash: stacks::BurnchainHeaderHash,
    pub sortition_id: stacks::SortitionId,
    pub parent_sortition_id: stacks::SortitionId,
    pub burn_header_timestamp: u64,
    pub parent_burn_header_hash: stacks::BurnchainHeaderHash,
    pub consensus_hash: stacks::ConsensusHash,
    pub ops_hash: stacks::OpsHash,
    pub total_burn: u64,
    pub is_sortition: bool,
    pub sortition_hash: stacks::SortitionHash,
    pub winning_block_txid: stacks::Txid,
    pub winning_stacks_block_hash: stacks::BlockHeaderHash,
    pub index_root: stacks::TrieHash,
    pub num_sortitions: u32,
    pub was_stacks_block_accepted: bool,
    pub stacks_block_height: u32,
    pub arrival_index: u32,
    pub canonical_stacks_tip_height: u32,
    pub canonical_stacks_tip_hash: stacks::BlockHeaderHash,
    pub canonical_stacks_tip_consensus_hash: stacks::ConsensusHash,
    pub is_pox_valid: bool,
    pub accumulated_coinbase_ustx: u64,
    pub pox_payouts: String,
}