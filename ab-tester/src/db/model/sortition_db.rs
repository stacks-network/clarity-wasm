use diesel::prelude::*;
use color_eyre::Result;

use crate::db::schema::sortition::*;
use crate::clarity;
use crate::stacks;
use crate::utils::*;

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(start_block_height, epoch_id))]
#[diesel(table_name = epochs)]
pub struct Epoch {
    pub start_block_height: i32,
    pub end_block_height: i32,
    pub epoch_id: i32,
    pub block_limit: String,
    pub network_epoch: i32,
}

impl From<Epoch> for clarity::StacksEpoch {
    fn from(value: Epoch) -> Self {
        clarity::StacksEpoch {
            start_height: value.start_block_height as u64,
            end_height: value.end_block_height as u64,
            epoch_id: (value.epoch_id as u32).try_into().expect("failed to convert epoch id from database to a StacksEpochId"),
            block_limit: serde_json::from_str(&value.block_limit)
                .expect("failed to deserialize block limit json"),
            network_epoch: value.network_epoch as u8
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(txid, sortition_id))]
#[diesel(table_name = block_commits)]
pub struct BlockCommit {
    pub txid: String,
    pub vtxindex: i32,
    pub block_height: i32,
    pub burn_header_hash: String,
    pub sortition_id: String,
    pub block_header_hash: String,
    pub new_seed: String,
    pub parent_block_ptr: i32,
    pub parent_vtxindex: i32,
    pub key_block_ptr: i32,
    pub key_vtxindex: i32,
    pub memo: String,
    pub commit_outs: String,
    pub burn_fee: String,
    pub sunset_burn: String,
    pub input: String,
    pub apparent_sender: String,
    pub burn_parent_modulus: i32,
}

impl TryFrom<BlockCommit> for crate::types::BlockCommit {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: BlockCommit) -> Result<Self> {
        Ok(Self {
            txid: stacks::Txid::from_hex(&value.txid)?,
            vtx_index: value.vtxindex as u32,
            block_height: value.block_height as u32,
            burn_header_hash: stacks::BurnchainHeaderHash::from_hex(&value.burn_header_hash)?,
            sortition_id: stacks::SortitionId::from_hex(&value.sortition_id)?,
            block_header_hash: stacks::BlockHeaderHash::from_hex(&value.block_header_hash)?,
            new_seed: stacks::VRFSeed::from_hex(&value.new_seed)?,
            parent_block_ptr: value.parent_block_ptr as u32,
            parent_vtx_index: value.parent_vtxindex as u32,
            key_block_ptr: value.key_block_ptr as u32,
            key_vtx_index: value.key_vtxindex as u32,
            memo: value.memo,
            commit_outs: serde_json::from_str(&value.commit_outs)?,
            burn_fee: value.burn_fee.parse()?,
            sunset_burn: value.sunset_burn.parse()?,
            input: serde_json::from_str(&value.input)?,
            apparent_sender: stacks::BurnchainSigner(value.apparent_sender),
            burn_parent_modulus: value.burn_parent_modulus as u32
        })
    }
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(sortition_id))]
#[diesel(table_name = snapshots)]
pub struct Snapshot {
    pub block_height: i32,
    pub burn_header_hash: String,
    pub sortition_id: String,
    pub parent_sortition_id: String,
    pub burn_header_timestamp: i64,
    pub parent_burn_header_hash: String,
    pub consensus_hash: String,
    pub ops_hash: String,
    pub total_burn: String,
    pub sortition: i32,
    pub sortition_hash: String,
    pub winning_block_txid: String,
    pub winning_stacks_block_hash: String,
    pub index_root: String,
    pub num_sortitions: i32,
    pub stacks_block_accepted: i32,
    pub stacks_block_height: i32,
    pub arrival_index: i32,
    pub canonical_stacks_tip_height: i32,
    pub canonical_stacks_tip_hash: String,
    pub canonical_stacks_tip_consensus_hash: String,
    pub pox_valid: i32,
    pub accumulated_coinbase_ustx: String,
    pub pox_payouts: String,
}

impl TryFrom<Snapshot> for crate::types::Snapshot {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: Snapshot) -> Result<Self> {
        Ok(Self {
            block_height: value.block_height as u32,
            burn_header_hash: stacks::BurnchainHeaderHash::from_hex(&value.burn_header_hash)?,
            sortition_id: stacks::SortitionId::from_hex(&value.sortition_id)?,
            parent_sortition_id: stacks::SortitionId::from_hex(&value.parent_sortition_id)?,
            burn_header_timestamp: value.burn_header_timestamp as u64,
            parent_burn_header_hash: stacks::BurnchainHeaderHash::from_hex(&value.parent_burn_header_hash)?,
            consensus_hash: stacks::ConsensusHash::from_hex(&value.consensus_hash)?,
            ops_hash: stacks::OpsHash::from_hex(&value.ops_hash)?,
            total_burn: value.total_burn.parse()?,
            is_sortition: try_convert_i32_to_bool(value.sortition)?,
            sortition_hash: stacks::SortitionHash::from_hex(&value.sortition_hash)?,
            winning_block_txid: stacks::Txid::from_hex(&value.winning_block_txid)?,
            winning_stacks_block_hash: stacks::BlockHeaderHash::from_hex(&value.winning_stacks_block_hash)?,
            index_root: stacks::TrieHash::from_hex(&value.index_root)?,
            num_sortitions: value.num_sortitions as u32,
            was_stacks_block_accepted: try_convert_i32_to_bool(value.stacks_block_accepted)?,
            stacks_block_height: value.stacks_block_height as u32,
            arrival_index: value.arrival_index as u32,
            canonical_stacks_tip_height: value.canonical_stacks_tip_height as u32,
            canonical_stacks_tip_hash: stacks::BlockHeaderHash::from_hex(&value.canonical_stacks_tip_hash)?,
            canonical_stacks_tip_consensus_hash: stacks::ConsensusHash::from_hex(&value.canonical_stacks_tip_consensus_hash)?,
            is_pox_valid: try_convert_i32_to_bool(value.pox_valid)?,
            accumulated_coinbase_ustx: value.accumulated_coinbase_ustx.parse()?,
            pox_payouts: value.pox_payouts
        })
    }
}

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(primary_key(ast_rule_id))]
#[diesel(table_name = ast_rule_heights)]
pub struct AstRuleHeight {
    pub ast_rule_id: i32,
    pub block_height: i32
}

impl From<AstRuleHeight> for clarity::ASTRules {
    fn from(value: AstRuleHeight) -> Self {
        match value.ast_rule_id {
            0 => clarity::ASTRules::Typical,
            1 => clarity::ASTRules::PrecheckSize,
            _ => panic!("failed to convert AstRuleHeight to clarity::ASTRules")
        }
    }
}