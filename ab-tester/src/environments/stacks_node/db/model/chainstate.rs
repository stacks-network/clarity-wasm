use color_eyre::eyre::anyhow;
use color_eyre::Result;
/// This file contains model types (DTOs) which represent tables in a Stacks
/// node's chainstate index DB, typically located in `chainstate/vm/index.sqlite`.
use diesel::prelude::*;

use super::super::schema::chainstate::*;
use crate::clarity;
use crate::stacks::{self, Address};

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(version))]
#[diesel(table_name = db_config)]
pub struct DbConfig {
    pub version: i32,
    pub mainnet: bool,
    pub chain_id: i32,
}

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
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

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(address, block_hash))]
#[diesel(table_name = payments)]
pub struct Payment {
    pub address: String,
    pub block_hash: String,
    pub burnchain_commit_burn: i32,
    pub burnchain_sortition_burn: i32,
}

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
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

impl TryFrom<BlockHeader> for crate::types::BlockHeader {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: BlockHeader) -> Result<Self> {
        Ok(Self {
            environment_id: 0,
            version: value.version as u32,
            total_burn: value.total_burn.parse()?,
            total_work: value.total_work.parse()?,
            proof: stacks::VRFProof::from_hex(&value.proof)
                .ok_or(anyhow!("failed to convert proof hex to VRFProof"))?,
            parent_block: stacks::BlockHeaderHash::from_hex(&value.parent_block)?,
            parent_microblock: stacks::BlockHeaderHash::from_hex(&value.parent_microblock)?,
            parent_microblock_sequence: value.parent_microblock_sequence as u32,
            tx_merkle_root: stacks::Sha512Trunc256Sum::from_hex(&value.tx_merkle_root)?,
            state_index_root: stacks::TrieHash::from_hex(&value.state_index_root)?,
            microblock_pubkey_hash: stacks::Hash160::from_hex(&value.microblock_pubkey_hash)?,
            block_hash: stacks::BlockHeaderHash::from_hex(&value.block_hash)?,
            index_block_hash: stacks::StacksBlockId::from_hex(&value.index_block_hash)?,
            block_height: value.block_height as u32,
            index_root: stacks::TrieHash::from_hex(&value.index_root)?,
            consensus_hash: stacks::ConsensusHash::from_hex(&value.consensus_hash)?,
            burn_header_hash: stacks::BurnchainHeaderHash::from_hex(&value.burn_header_hash)?,
            burn_header_height: value.burn_header_height as u32,
            burn_header_timestamp: value.burn_header_timestamp as u64,
            parent_block_id: stacks::StacksBlockId::from_hex(&value.parent_block_id)?,
            cost: serde_json::from_str(&value.cost)?,
            block_size: value.block_size.parse()?,
            affirmation_weight: value.affirmation_weight as u64,
        })
    }
}

impl TryFrom<crate::types::BlockHeader> for BlockHeader {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::BlockHeader) -> Result<Self> {
        Ok(Self {
            version: value.version as i32,
            total_burn: value.total_burn.to_string(),
            total_work: value.total_work.to_string(),
            proof: value.proof.to_hex(),
            parent_block: value.parent_block.to_hex(),
            parent_microblock: value.parent_microblock.to_hex(),
            parent_microblock_sequence: value.parent_microblock_sequence as i32,
            tx_merkle_root: value.tx_merkle_root.to_hex(),
            state_index_root: value.state_index_root.to_hex(),
            microblock_pubkey_hash: value.microblock_pubkey_hash.to_hex(),
            block_hash: value.block_hash.to_hex(),
            index_block_hash: value.index_block_hash.to_hex(),
            block_height: value.block_height as i32,
            index_root: value.index_root.to_hex(),
            consensus_hash: value.consensus_hash.to_hex(),
            burn_header_hash: value.burn_header_hash.to_hex(),
            burn_header_height: value.burn_header_height as i32,
            burn_header_timestamp: value.burn_header_timestamp as i64,
            parent_block_id: value.parent_block_id.to_hex(),
            cost: serde_json::to_string(&value.cost)?,
            block_size: value.block_size.to_string(),
            affirmation_weight: value.affirmation_weight as i32,
        })
    }
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
