use crate::stacks;

#[derive(Debug, Clone)]
pub struct Payment {
    pub environment_id: i32,
    pub address: stacks::StacksAddress,
    pub block_hash: stacks::BlockHeaderHash,
    pub burnchain_commit_burn: u32,
    pub burnchain_sortition_burn: u32,
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub environment_id: i32,
    pub version: u32,
    pub total_burn: u64,
    pub total_work: u64,
    pub proof: stacks::VRFProof,
    /// Hash of parent Stacks block.
    pub parent_block: stacks::BlockHeaderHash,
    pub parent_microblock: stacks::BlockHeaderHash,
    pub parent_microblock_sequence: u32,
    pub tx_merkle_root: stacks::Sha512Trunc256Sum,
    pub state_index_root: stacks::TrieHash,
    pub microblock_pubkey_hash: stacks::Hash160,
    /// Note: this is *not* unique, since two burn chain forks can commit
    /// to the same Stacks block.
    pub block_hash: stacks::BlockHeaderHash,
    /// Note: this is the hash of the block hash and consensus hash of the
    /// burn block that selected it, and is guaranteed to be globally unique
    /// (across all Stacks forks and across all PoX forks).
    /// index_block_hash is the block hash fed into the MARF index.
    pub index_block_hash: stacks::StacksBlockId,
    pub block_height: u32,
    /// Root hash of the internal, not-conensus-critical MARF that allows
    /// us to track chainstate/fork metadata.
    pub index_root: stacks::TrieHash,
    /// All consensus hashes are guaranteed to be unique.
    pub consensus_hash: stacks::ConsensusHash,
    /// Burn header hash corresponding to the consensus hash (NOT guaranteed
    /// to be unique, since we can have 2+ blocks per burn block if there's
    /// a PoX fork).
    pub burn_header_hash: stacks::BurnchainHeaderHash,
    /// Height of the burnchain block header that generated this consensus hash.
    pub burn_header_height: u32,
    /// Timestamp from the burnchain block header that generated this consensus hash.
    pub burn_header_timestamp: u64,
    /// NOTE: this is the parent index_block_hash.
    pub parent_block_id: stacks::StacksBlockId,
    pub cost: u64,
    /// Converted to/from u64.
    pub block_size: u64,
    pub affirmation_weight: u64,
}

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
