use diesel::prelude::*;
use serde;
use serde::Deserialize;

use crate::db::schema::sortition::*;
use crate::clarity;
use crate::stacks;

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
    pub burn_parent_modules: i32,
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
    pub sortition: bool,
    pub sortition_hash: String,
    pub winning_block_txid: String,
    pub winning_stacks_block_hash: String,
    pub index_root: String,
    pub num_sortitions: i32,
    pub stacks_block_accepted: bool,
    pub stacks_block_height: i32,
    pub arrival_index: i32,
    pub canonical_stacks_tip_height: i32,
    pub canonical_stacks_tip_hash: String,
    pub canonical_stacks_tip_consensus_hash: String,
    pub pox_valid: bool,
    pub accumulated_coinbase_ustx: String,
    pub pox_payouts: String,
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