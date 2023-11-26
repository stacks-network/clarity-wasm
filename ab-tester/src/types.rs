use crate::stacks;

#[derive(Debug, Clone)]
pub struct AstRuleHeight {
    pub environment_id: i32,
    pub ast_rule_id: u32,
    pub block_height: u32,
}

#[derive(Debug, Clone)]
pub struct Epoch {
    pub environment_id: i32,
    pub start_block_height: u32,
    pub end_block_height: u32,
    pub epoch_id: stacks::StacksEpochId,
    pub block_limit: stacks::ExecutionCost,
    pub network_epoch: u32,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub environment_id: i32,
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
    pub pox_payouts: (Vec<stacks::PoxAddress>, u128),
}

#[derive(Debug, Clone)]
pub struct BlockCommit {
    pub environment_id: i32,
    pub txid: stacks::Txid,
    pub vtx_index: u32,
    pub block_height: u32,
    pub burn_header_hash: stacks::BurnchainHeaderHash,
    pub sortition_id: stacks::SortitionId,
    pub block_header_hash: stacks::BlockHeaderHash,
    pub new_seed: stacks::VRFSeed,
    pub parent_block_ptr: u32,
    pub parent_vtx_index: u32,
    pub key_block_ptr: u32,
    pub key_vtx_index: u32,
    pub memo: String,
    pub commit_outs: Vec<stacks::PoxAddress>,
    pub burn_fee: u64,
    pub sunset_burn: u64,
    pub input: (stacks::Txid, u32),
    pub apparent_sender: stacks::BurnchainSigner,
    pub burn_parent_modulus: u32,
}