use diesel::prelude::*;

use crate::clarity;
use crate::db::schema::chainstate_marf::*;
use crate::stacks::{self, Address};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(version))]
#[diesel(table_name = db_config)]
pub struct DbConfig {
    pub version: i32,
    pub mainnet: bool,
    pub chain_id: i32,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(parent_index_block_hash, child_index_block_hash, coinbase))]
#[diesel(table_name = matured_rewards)]
pub struct MaturedReward {
    pub address: String,
    pub recipient: String,
    pub vtxindex: i32,
    pub coinbase: String,
    pub tx_fees_anchored: String,
    pub tx_fees_streamed_confirmed: String,
    pub tx_fees_streamed_produced: String,
    pub child_index_block_hash: String,
    pub parent_index_block_hash: String,
}

impl From<&MaturedReward> for stacks::MinerReward {
    fn from(val: &MaturedReward) -> Self {
        stacks::MinerReward {
            address: stacks::StacksAddress::from_string(&val.address)
                .expect("FATAL: could not parse miner address"),
            recipient: clarity::PrincipalData::parse(&val.recipient)
                .expect("FATAL: could not parse recipient principal"),
            vtxindex: val.vtxindex as u32,
            coinbase: val
                .coinbase
                .parse::<u128>()
                .expect("FATAL: Failed to convert coinbase to u128"),
            tx_fees_anchored: val
                .tx_fees_anchored
                .parse::<u128>()
                .expect("FATAL: Failed to convert tx_fees_anchored to u128"),
            tx_fees_streamed_confirmed: val
                .tx_fees_streamed_confirmed
                .parse::<u128>()
                .expect("FATAL: Failed to convert tx_fees_streamed_confirmed to u128"),
            tx_fees_streamed_produced: val
                .tx_fees_streamed_produced
                .parse::<u128>()
                .expect("FATAL: Failed to convert tx_fees_streamed_produced to u128"),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(address, block_hash))]
#[diesel(table_name = payments)]
pub struct Payment {
    pub address: String,
    pub block_hash: String,
    pub burnchain_commit_burn: i32,
    pub burnchain_sortition_burn: i32,
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(consensus_hash, block_hash))]
#[diesel(table_name = block_headers)]
pub struct BlockHeader {
    pub version: i32,
    /// Converted to/from u64
    pub total_burn: String,
    /// Converted to/from u64
    pub total_work: String,
    pub proof: String,
    /// Hash of parent Stacks block.
    pub parent_block: String,
    pub parent_microblock: String,
    pub parent_microblock_sequence: i32,
    pub tx_merkle_root: String,
    pub state_index_root: String,
    pub microblock_pubkey_hash: String,
    /// Note: this is *not* unique, since two burn chain forks can commit
    /// to the same Stacks block.
    pub block_hash: String,
    /// Note: this is the hash of the block hash and consensus hash of the
    /// burn block that selected it, and is guaranteed to be globally unique
    /// (across all Stacks forks and across all PoX forks).
    /// index_block_hash is the block hash fed into the MARF index.
    pub index_block_hash: String,
    pub block_height: i32,
    /// Root hash of the internal, not-conensus-critical MARF that allows
    /// us to track chainstate/fork metadata.
    pub index_root: String,
    /// All consensus hashes are guaranteed to be unique.
    pub consensus_hash: String,
    /// Burn header hash corresponding to the consensus hash (NOT guaranteed
    /// to be unique, since we can have 2+ blocks per burn block if there's
    /// a PoX fork).
    pub burn_header_hash: String,
    /// Height of the burnchain block header that generated this consensus hash.
    pub burn_header_height: i32,
    /// Timestamp from the burnchain block header that generated this consensus hash.
    pub burn_header_timestamp: i64,
    /// NOTE: this is the parent index_block_hash.
    pub parent_block_id: String,
    pub cost: String,
    /// Converted to/from u64.
    pub block_size: String,
    pub affirmation_weight: i32,
}

impl BlockHeader {
    pub fn block_height(&self) -> u32 {
        self.block_height as u32
    }
}

impl BlockHeader {
    pub fn is_genesis(&self) -> bool {
        self.block_height == 0
    }
}
