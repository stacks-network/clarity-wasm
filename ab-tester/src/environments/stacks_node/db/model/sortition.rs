/// This file represents the database model for the RDBMS storage of a Stacks
/// node's Sortition DB (typically located in `burnstate/sortition/marf.sqlite`).
use color_eyre::eyre::anyhow;
use color_eyre::Result;
use diesel::prelude::*;

use super::super::schema::sortition::*;
use crate::utils::*;
use crate::{clarity, stacks};

/// The sortition DB model's [Epoch] instance. Model for table `epochs` in the
/// sortition DB's SQL database.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(start_block_height, epoch_id))]
#[diesel(table_name = epochs)]
pub struct Epoch {
    pub start_block_height: i32,
    pub end_block_height: i32,
    pub epoch_id: i32,
    pub block_limit: String,
    pub network_epoch: i32,
}

/// Convert from the sortition DB model's [Epoch] to a Clarity VM
/// [clarity::StacksEpoch] instance.
impl From<Epoch> for clarity::StacksEpoch {
    fn from(value: Epoch) -> Self {
        clarity::StacksEpoch {
            start_height: value.start_block_height as u64,
            end_height: value.end_block_height as u64,
            epoch_id: (value.epoch_id as u32)
                .try_into()
                .expect("failed to convert epoch id from database to a StacksEpochId"),
            block_limit: serde_json::from_str(&value.block_limit)
                .expect("failed to deserialize block limit json"),
            network_epoch: value.network_epoch as u8,
        }
    }
}

/// Convert from the sortition DB model's [Epoch] to the DTO type [crate::types::Epoch].
impl TryFrom<Epoch> for crate::types::Epoch {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: Epoch) -> Result<Self> {
        Ok(Self {
            environment_id: 0,
            start_block_height: value.start_block_height as u32,
            end_block_height: value.end_block_height as u32,
            epoch_id: (value.epoch_id as u32)
                .try_into()
                .map_err(|e| anyhow!("{e:?}"))?,
            block_limit: serde_json::from_str(&value.block_limit)?,
            network_epoch: value.network_epoch as u32,
        })
    }
}

/// Convert from the DTO type [crate::types::Epoch] to the sortition DB model's
/// [Epoch].
impl TryFrom<crate::types::Epoch> for Epoch {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::Epoch) -> Result<Self> {
        Ok(Self {
            start_block_height: value.start_block_height as i32,
            end_block_height: value.end_block_height as i32,
            epoch_id: value.epoch_id as i32,
            block_limit: serde_json::to_string(&value.block_limit)?,
            network_epoch: value.network_epoch as i32,
        })
    }
}

/// The sortition DB model's [BlockCommit] instance. Model for the table `block_commits`
/// in the sortition DB's SQL database.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
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

/// Convert from the sortition DB model's [BlockCommit] to the DTO type
/// [crate::types::BlockCommit].
impl TryFrom<BlockCommit> for crate::types::BlockCommit {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: BlockCommit) -> Result<Self> {
        Ok(Self {
            environment_id: 0,
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
            burn_parent_modulus: value.burn_parent_modulus as u32,
        })
    }
}

impl TryFrom<crate::types::BlockCommit> for BlockCommit {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::BlockCommit) -> Result<Self> {
        Ok(Self {
            txid: value.txid.to_hex(),
            vtxindex: value.vtx_index as i32,
            block_height: value.block_height as i32,
            burn_header_hash: value.burn_header_hash.to_hex(),
            sortition_id: value.sortition_id.to_hex(),
            block_header_hash: value.block_header_hash.to_hex(),
            new_seed: value.new_seed.to_hex(),
            parent_block_ptr: value.parent_block_ptr as i32,
            parent_vtxindex: value.parent_vtx_index as i32,
            key_block_ptr: value.key_block_ptr as i32,
            key_vtxindex: value.key_vtx_index as i32,
            memo: value.memo,
            commit_outs: serde_json::to_string(&value.commit_outs)?,
            burn_fee: value.burn_fee.to_string(),
            sunset_burn: value.sunset_burn.to_string(),
            input: serde_json::to_string(&value.input)?,
            apparent_sender: value.apparent_sender.to_string(),
            burn_parent_modulus: value.burn_parent_modulus as i32,
        })
    }
}

/// The sortition DB model's [Snapshot] instance. Model for table `snapshots` in the
/// sortition DB's SQL database.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
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

/// Convert from the sortition DB model's [Snapshot] to the DTO type
/// [crate::types::Snapshot].
impl TryFrom<Snapshot> for crate::types::Snapshot {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: Snapshot) -> Result<Self> {
        Ok(Self {
            environment_id: 0,
            block_height: value.block_height as u32,
            burn_header_hash: stacks::BurnchainHeaderHash::from_hex(&value.burn_header_hash)?,
            sortition_id: stacks::SortitionId::from_hex(&value.sortition_id)?,
            parent_sortition_id: stacks::SortitionId::from_hex(&value.parent_sortition_id)?,
            burn_header_timestamp: value.burn_header_timestamp as u64,
            parent_burn_header_hash: stacks::BurnchainHeaderHash::from_hex(
                &value.parent_burn_header_hash,
            )?,
            consensus_hash: stacks::ConsensusHash::from_hex(&value.consensus_hash)?,
            ops_hash: stacks::OpsHash::from_hex(&value.ops_hash)?,
            total_burn: value.total_burn.parse()?,
            is_sortition: try_convert_i32_to_bool(value.sortition)?,
            sortition_hash: stacks::SortitionHash::from_hex(&value.sortition_hash)?,
            winning_block_txid: stacks::Txid::from_hex(&value.winning_block_txid)?,
            winning_stacks_block_hash: stacks::BlockHeaderHash::from_hex(
                &value.winning_stacks_block_hash,
            )?,
            index_root: stacks::TrieHash::from_hex(&value.index_root)?,
            num_sortitions: value.num_sortitions as u32,
            was_stacks_block_accepted: try_convert_i32_to_bool(value.stacks_block_accepted)?,
            stacks_block_height: value.stacks_block_height as u32,
            arrival_index: value.arrival_index as u32,
            canonical_stacks_tip_height: value.canonical_stacks_tip_height as u32,
            canonical_stacks_tip_hash: stacks::BlockHeaderHash::from_hex(
                &value.canonical_stacks_tip_hash,
            )?,
            canonical_stacks_tip_consensus_hash: stacks::ConsensusHash::from_hex(
                &value.canonical_stacks_tip_consensus_hash,
            )?,
            is_pox_valid: try_convert_i32_to_bool(value.pox_valid)?,
            accumulated_coinbase_ustx: value.accumulated_coinbase_ustx.parse()?,
            pox_payouts: serde_json::from_str(&value.pox_payouts)?,
        })
    }
}

/// Convert from the DTO type [crate::types::Snapshot] to the sortition DB
/// model's [Snapshot].
impl TryFrom<crate::types::Snapshot> for Snapshot {
    type Error = color_eyre::eyre::Error;
    fn try_from(value: crate::types::Snapshot) -> Result<Self> {
        Ok(Self {
            block_height: value.block_height as i32,
            burn_header_hash: value.burn_header_hash.to_hex(),
            sortition_id: value.sortition_id.to_hex(),
            parent_sortition_id: value.parent_sortition_id.to_hex(),
            burn_header_timestamp: value.burn_header_timestamp as i64,
            parent_burn_header_hash: value.parent_burn_header_hash.to_hex(),
            consensus_hash: value.consensus_hash.to_hex(),
            ops_hash: value.ops_hash.to_hex(),
            total_burn: value.total_burn.to_string(),
            sortition: value.is_sortition.into(),
            sortition_hash: value.sortition_hash.to_hex(),
            winning_block_txid: value.winning_block_txid.to_hex(),
            winning_stacks_block_hash: value.winning_stacks_block_hash.to_hex(),
            index_root: value.index_root.to_string(),
            num_sortitions: value.num_sortitions as i32,
            stacks_block_accepted: value.was_stacks_block_accepted as i32,
            stacks_block_height: value.stacks_block_height as i32,
            arrival_index: value.arrival_index as i32,
            canonical_stacks_tip_height: value.canonical_stacks_tip_height as i32,
            canonical_stacks_tip_hash: value.canonical_stacks_tip_hash.to_hex(),
            canonical_stacks_tip_consensus_hash: value.canonical_stacks_tip_consensus_hash.to_hex(),
            pox_valid: value.is_pox_valid as i32,
            accumulated_coinbase_ustx: value.accumulated_coinbase_ustx.to_string(),
            pox_payouts: serde_json::to_string(&value.pox_payouts)?,
        })
    }
}

/// The sortition DB model's [AstRuleHeight] instance. Model for table `ast_rule_heights`
/// in the sortition DB's SQL database.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(ast_rule_id))]
#[diesel(table_name = ast_rule_heights)]
pub struct AstRuleHeight {
    pub ast_rule_id: i32,
    pub block_height: i32,
}

/// Convert from the sortition DB model's [AstRuleHeight] to the Clarity VM
/// type [clarity::ASTRules].
impl From<AstRuleHeight> for clarity::ASTRules {
    fn from(value: AstRuleHeight) -> Self {
        match value.ast_rule_id {
            0 => clarity::ASTRules::Typical,
            1 => clarity::ASTRules::PrecheckSize,
            _ => panic!("failed to convert AstRuleHeight to clarity::ASTRules"),
        }
    }
}

/// Convert from the sortition DB model's [AstRuleHeight] to the DTO type
/// [crate::types::AstRuleHeight].
impl TryFrom<AstRuleHeight> for crate::types::AstRuleHeight {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: AstRuleHeight) -> Result<Self> {
        Ok(Self {
            environment_id: 0,
            ast_rule_id: value.ast_rule_id as u32,
            block_height: value.block_height as u32,
        })
    }
}

/// Convert from the DTO type [crate::types::AstRuleHeight] to the sortition
/// DB model's [AstRuleHeight].
impl TryFrom<crate::types::AstRuleHeight> for AstRuleHeight {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::AstRuleHeight) -> Result<Self> {
        Ok(Self {
            ast_rule_id: value.ast_rule_id as i32,
            block_height: value.block_height as i32,
        })
    }
}
