use color_eyre::eyre::{anyhow, bail};
/// This file contains model objects (DTOs) which represent this application's
/// persistent state which is stored in an RDBMS.
use color_eyre::Result;
use diesel::prelude::*;

use super::schema::*;
use crate::stacks::Address;
use crate::{clarity, stacks};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName)]
#[diesel(table_name = runtime)]
pub struct Runtime {
    pub id: i32,
    pub name: String,
}

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = environment)]
pub struct Environment {
    pub id: i32,
    pub name: String,
    pub runtime_id: i32,
    pub path: String,
}

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = block)]
pub struct Block {
    pub id: i32,
    pub environment_id: i32,
    //pub stacks_block_id: i32,
    pub height: i32,
    pub index_hash: Vec<u8>,
    pub marf_trie_root_hash: Vec<u8>,
}

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = marf_entry)]
pub struct MarfEntry {
    pub id: i32,
    pub block_id: i32,
    pub key_hash: Vec<u8>,
    pub value: Vec<u8>,
}

/// A generalized instance to an installed Clarity contract.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = contract)]
pub struct Contract {
    pub id: i32,
    pub block_id: i32,
    pub qualified_contract_id: String,
    pub source: Vec<u8>,
}

/// Holds information about a specific execution of a Clarity contract.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = contract_execution)]
pub struct ContractExecution {
    pub id: i32,
    pub block_id: i32,
    pub contract_id: i32,
    pub transaction_id: Vec<u8>,
}

/// A data-var definition for a Clarity contract.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = contract_var)]
pub struct ContractVar {
    pub id: i32,
    pub contract_id: i32,
    pub key: String,
}

/// A single Clarity data-var instance which is associated with a specific contract
/// execution.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = contract_var_instance)]
pub struct ContractVarInstance {
    pub id: i32,
    pub contract_var_id: i32,
    pub contract_execution_id: i32,
    pub value: Vec<u8>,
}

/// Information regarding Clarity maps in a contract.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(table_name = contract_map)]
pub struct ContractMap {
    pub id: i32,
    pub contract_id: i32,
    pub name: String,
}

#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(environment_id, consensus_hash, block_hash))]
#[diesel(table_name = _block_headers)]
pub struct BlockHeader {
    pub environment_id: i32,
    pub version: i32,
    /// Converted to/from u64
    pub total_burn: i64,
    /// Converted to/from u64
    pub total_work: i64,
    pub proof: Vec<u8>,
    /// Hash of parent Stacks block.
    pub parent_block: Vec<u8>,
    pub parent_microblock: Vec<u8>,
    pub parent_microblock_sequence: i32,
    pub tx_merkle_root: Vec<u8>,
    pub state_index_root: Vec<u8>,
    pub microblock_pubkey_hash: Vec<u8>,
    /// Note: this is *not* unique, since two burn chain forks can commit
    /// to the same Stacks block.
    pub block_hash: Vec<u8>,
    /// Note: this is the hash of the block hash and consensus hash of the
    /// burn block that selected it, and is guaranteed to be globally unique
    /// (across all Stacks forks and across all PoX forks).
    /// index_block_hash is the block hash fed into the MARF index.
    pub index_block_hash: Vec<u8>,
    pub block_height: i32,
    /// Root hash of the internal, not-conensus-critical MARF that allows
    /// us to track chainstate/fork metadata.
    pub index_root: Vec<u8>,
    /// All consensus hashes are guaranteed to be unique.
    pub consensus_hash: Vec<u8>,
    /// Burn header hash corresponding to the consensus hash (NOT guaranteed
    /// to be unique, since we can have 2+ blocks per burn block if there's
    /// a PoX fork).
    pub burn_header_hash: Vec<u8>,
    /// Height of the burnchain block header that generated this consensus hash.
    pub burn_header_height: i32,
    /// Timestamp from the burnchain block header that generated this consensus hash.
    pub burn_header_timestamp: i64,
    /// NOTE: this is the parent index_block_hash.
    pub parent_block_id: Vec<u8>,
    pub cost: Vec<u8>,
    /// Converted to/from u64.
    pub block_size: i64,
    pub affirmation_weight: i32,
}

impl TryFrom<BlockHeader> for crate::types::BlockHeader {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: BlockHeader) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            version: value.version as u32,
            total_burn: value.total_burn as u64,
            total_work: value.total_work as u64,
            proof: stacks::VRFProof::from_bytes(&value.proof)
                .ok_or(anyhow!("failed to convert proof into VRFProof"))?,
            parent_block: stacks::BlockHeaderHash::from_bytes(&value.parent_block).ok_or(
                anyhow!("failed to convert parent_block into BlockHeaderHash"),
            )?,
            parent_microblock: stacks::BlockHeaderHash::from_bytes(&value.parent_microblock)
                .ok_or(anyhow!(
                    "failed to convert parent_microblock into BlockHeaderHash"
                ))?,
            parent_microblock_sequence: value.parent_microblock_sequence as u32,
            tx_merkle_root: stacks::Sha512Trunc256Sum::from_bytes(&value.tx_merkle_root).ok_or(
                anyhow!("failed to convert tx_merkle_root into Sha512Trunc256Sum"),
            )?,
            state_index_root: stacks::TrieHash::from_bytes(&value.state_index_root)
                .ok_or(anyhow!("failed to convert state_index_root into TrieHash"))?,
            microblock_pubkey_hash: stacks::Hash160::from_bytes(&value.microblock_pubkey_hash)
                .ok_or(anyhow!(
                    "failed to convert microblock_pubkey_hash into Hash160"
                ))?,
            block_hash: stacks::BlockHeaderHash::from_bytes(&value.block_hash)
                .ok_or(anyhow!("failed to convert block_hash into BlockHeaderHash"))?,
            index_block_hash: stacks::StacksBlockId::from_bytes(&value.index_block_hash).ok_or(
                anyhow!("failed to convert index_block_hash into StacksBlockId"),
            )?,
            block_height: value.block_height as u32,
            index_root: stacks::TrieHash::from_bytes(&value.index_root)
                .ok_or(anyhow!("failed to convert index_root into TrieHash"))?,
            consensus_hash: stacks::ConsensusHash::from_bytes(&value.consensus_hash).ok_or(
                anyhow!("failed to convert consensus_hash into ConsensusHash"),
            )?,
            burn_header_hash: stacks::BurnchainHeaderHash::from_bytes(&value.burn_header_hash)
                .ok_or(anyhow!(
                    "failed to convert burn_header_hash into BurnchainHeaderHash"
                ))?,
            burn_header_height: value.burn_header_height as u32,
            burn_header_timestamp: value.burn_header_timestamp as u64,
            parent_block_id: stacks::StacksBlockId::from_bytes(&value.parent_block_id).ok_or(
                anyhow!("failed to convert parent_block_id into StacksBlockId"),
            )?,
            cost: rmp_serde::decode::from_slice(&value.cost)?,
            block_size: value.block_size as u64,
            affirmation_weight: value.affirmation_weight as u64,
        })
    }
}

impl TryFrom<crate::types::BlockHeader> for BlockHeader {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::BlockHeader) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            version: value.version as i32,
            total_burn: value.total_burn as i64,
            total_work: value.total_work as i64,
            proof: value.proof.to_bytes().to_vec(),
            parent_block: value.parent_block.0.to_vec(),
            parent_microblock: value.parent_microblock.0.to_vec(),
            parent_microblock_sequence: value.parent_microblock_sequence as i32,
            tx_merkle_root: value.tx_merkle_root.0.to_vec(),
            state_index_root: value.state_index_root.0.to_vec(),
            microblock_pubkey_hash: value.microblock_pubkey_hash.0.to_vec(),
            block_hash: value.block_hash.0.to_vec(),
            index_block_hash: value.index_block_hash.0.to_vec(),
            block_height: value.block_height as i32,
            index_root: value.index_root.0.to_vec(),
            consensus_hash: value.consensus_hash.0.to_vec(),
            burn_header_hash: value.burn_header_hash.0.to_vec(),
            burn_header_height: value.burn_header_height as i32,
            burn_header_timestamp: value.burn_header_timestamp as i64,
            parent_block_id: value.parent_block_id.0.to_vec(),
            cost: rmp_serde::to_vec(&value.cost)?,
            block_size: value.block_size as i64,
            affirmation_weight: value.affirmation_weight as i32,
        })
    }
}

/// Represents the `payments` table in a Stacks node's chainstate index db.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(environment_id, address, block_hash))]
#[diesel(table_name = _payments)]
pub struct Payment {
    pub environment_id: i32,
    pub address: String,
    pub block_hash: Vec<u8>,
    pub burnchain_commit_burn: i32,
    pub burnchain_sortition_burn: i32,
}

/// Represents the `matured_rewards` table in a Stacks node's chainstate index db.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(
    environment_id,
    parent_index_block_hash,
    child_index_block_hash,
    coinbase
))]
#[diesel(table_name = _matured_rewards)]
pub struct MaturedReward {
    pub environment_id: i32,
    pub address: String,
    pub recipient: String,
    pub vtxindex: i32,
    pub coinbase: i64,
    pub tx_fees_anchored: i32,
    pub tx_fees_streamed_confirmed: i32,
    pub tx_fees_streamed_produced: i32,
    pub child_index_block_hash: Vec<u8>,
    pub parent_index_block_hash: Vec<u8>,
}

impl From<&MaturedReward> for stacks::MinerReward {
    fn from(val: &MaturedReward) -> Self {
        stacks::MinerReward {
            address: stacks::StacksAddress::from_string(&val.address)
                .expect("FATAL: could not parse miner address"),
            recipient: clarity::PrincipalData::parse(&val.recipient)
                .expect("FATAL: could not parse recipient principal"),
            vtxindex: val.vtxindex as u32,
            coinbase: val.coinbase as u128,
            tx_fees_anchored: val.tx_fees_anchored as u128,
            tx_fees_streamed_confirmed: val.tx_fees_streamed_confirmed as u128,
            tx_fees_streamed_produced: val.tx_fees_streamed_produced as u128,
        }
    }
}

/// Represents the `ast_rule_heights` table in a Stacks node's sortition db.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(environment_id, ast_rule_id))]
#[diesel(table_name = _ast_rule_heights)]
pub struct AstRuleHeight {
    pub environment_id: i32,
    pub ast_rule_id: i32,
    pub block_height: i32,
}

impl TryFrom<crate::types::AstRuleHeight> for AstRuleHeight {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::AstRuleHeight) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            ast_rule_id: value.ast_rule_id as i32,
            block_height: value.block_height as i32,
        })
    }
}

/// Convert from the sortition DB model's [AstRuleHeight] to the Clarity VM
/// type [clarity::ASTRules].
impl TryFrom<AstRuleHeight> for clarity::ASTRules {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: AstRuleHeight) -> Result<Self> {
        match value.ast_rule_id {
            0 => Ok(clarity::ASTRules::Typical),
            1 => Ok(clarity::ASTRules::PrecheckSize),
            _ => bail!("failed to convert AstRuleHeight to clarity::ASTRules"),
        }
    }
}

/// Represents the `epochs` table in a Stacks node's sortition db.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(environment_id, start_block_height, epoch_id))]
#[diesel(table_name = _epochs)]
pub struct Epoch {
    pub environment_id: i32,
    pub start_block_height: i32,
    pub end_block_height: i32,
    pub epoch_id: i32,
    pub block_limit: Vec<u8>,
    pub network_epoch: i32,
}

impl TryFrom<crate::types::Epoch> for Epoch {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::Epoch) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            start_block_height: value.start_block_height as i32,
            end_block_height: value.end_block_height as i32,
            epoch_id: value.epoch_id as i32,
            block_limit: rmp_serde::encode::to_vec(&value.block_limit)?,
            network_epoch: value.network_epoch as i32,
        })
    }
}

impl TryFrom<Epoch> for clarity::StacksEpoch {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: Epoch) -> Result<Self> {
        Ok(Self {
            start_height: value.start_block_height as u64,
            end_height: value.end_block_height as u64,
            epoch_id: (value.epoch_id as u32).try_into().map_err(|e| {
                anyhow!("failed to convert epoch id from database to a StacksEpochId: {e}")
            })?,
            block_limit: rmp_serde::decode::from_slice(&value.block_limit)?,
            network_epoch: value.network_epoch as u8,
        })
    }
}

/// Represents the `block_commits` table in a Stacks node's sortition db.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(environment_id, txid, sortition_id))]
#[diesel(table_name = _block_commits)]
pub struct BlockCommit {
    pub environment_id: i32,
    pub txid: Vec<u8>,
    pub vtxindex: i32,
    pub block_height: i32,
    pub burn_header_hash: Vec<u8>,
    pub sortition_id: Vec<u8>,
    pub block_header_hash: Vec<u8>,
    pub new_seed: Vec<u8>,
    pub parent_block_ptr: i32,
    pub parent_vtxindex: i32,
    pub key_block_ptr: i32,
    pub key_vtxindex: i32,
    pub memo: String,
    pub commit_outs: Vec<u8>,
    pub burn_fee: i64,
    pub sunset_burn: i64,
    pub input: Vec<u8>,
    pub apparent_sender: Vec<u8>,
    pub burn_parent_modulus: i32,
}

impl TryFrom<crate::types::BlockCommit> for BlockCommit {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::BlockCommit) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            txid: value.txid.0.to_vec(),
            vtxindex: value.vtx_index as i32,
            block_height: value.block_height as i32,
            burn_header_hash: value.burn_header_hash.0.to_vec(),
            sortition_id: value.sortition_id.0.to_vec(),
            block_header_hash: value.block_header_hash.0.to_vec(),
            new_seed: value.new_seed.0.to_vec(),
            parent_block_ptr: value.parent_block_ptr as i32,
            parent_vtxindex: value.parent_vtx_index as i32,
            key_block_ptr: value.key_block_ptr as i32,
            key_vtxindex: value.key_vtx_index as i32,
            memo: value.memo,
            commit_outs: rmp_serde::encode::to_vec(&value.commit_outs)?,
            burn_fee: value.burn_fee as i64,
            sunset_burn: value.sunset_burn as i64,
            input: rmp_serde::encode::to_vec(&value.input)?,
            apparent_sender: rmp_serde::encode::to_vec(&value.apparent_sender)?,
            burn_parent_modulus: value.burn_parent_modulus as i32,
        })
    }
}

impl TryFrom<BlockCommit> for crate::types::BlockCommit {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: BlockCommit) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            txid: stacks::Txid(value.txid.try_into().map_err(|e| anyhow!("{:?}", e))?),
            vtx_index: value.vtxindex as u32,
            block_height: value.block_height as u32,
            burn_header_hash: stacks::BurnchainHeaderHash(
                value
                    .burn_header_hash
                    .try_into()
                    .map_err(|e| anyhow!("{:?}", e))?,
            ),
            sortition_id: stacks::SortitionId(
                value
                    .sortition_id
                    .try_into()
                    .map_err(|e| anyhow!("{:?}", e))?,
            ),
            block_header_hash: stacks::BlockHeaderHash(
                value
                    .block_header_hash
                    .try_into()
                    .map_err(|e| anyhow!("{:?}", e))?,
            ),
            new_seed: stacks::VRFSeed(value.new_seed.try_into().map_err(|e| anyhow!("{:?}", e))?),
            parent_block_ptr: value.parent_block_ptr as u32,
            parent_vtx_index: value.parent_vtxindex as u32,
            key_block_ptr: value.key_block_ptr as u32,
            key_vtx_index: value.key_vtxindex as u32,
            memo: value.memo,
            commit_outs: rmp_serde::decode::from_slice(&value.commit_outs)?,
            burn_fee: value.burn_fee as u64,
            sunset_burn: value.sunset_burn as u64,
            input: rmp_serde::decode::from_slice(&value.input)?,
            apparent_sender: rmp_serde::decode::from_slice(&value.apparent_sender)?,
            burn_parent_modulus: value.burn_parent_modulus as u32,
        })
    }
}

/// Represents the `snapshots` table in a Stacks node's sortition db.
#[derive(
    Queryable, Selectable, Identifiable, PartialEq, Eq, Debug, Clone, QueryableByName, Insertable,
)]
#[diesel(primary_key(environment_id, sortition_id))]
#[diesel(table_name = _snapshots)]
pub struct Snapshot {
    pub environment_id: i32,
    pub block_height: i32,
    pub burn_header_hash: Vec<u8>,
    pub sortition_id: Vec<u8>,
    pub parent_sortition_id: Vec<u8>,
    pub burn_header_timestamp: i64,
    pub parent_burn_header_hash: Vec<u8>,
    pub consensus_hash: Vec<u8>,
    pub ops_hash: Vec<u8>,
    pub total_burn: i64,
    pub sortition: bool,
    pub sortition_hash: Vec<u8>,
    pub winning_block_txid: Vec<u8>,
    pub winning_stacks_block_hash: Vec<u8>,
    pub index_root: Vec<u8>,
    pub num_sortitions: i32,
    pub stacks_block_accepted: bool,
    pub stacks_block_height: i32,
    pub arrival_index: i32,
    pub canonical_stacks_tip_height: i32,
    pub canonical_stacks_tip_hash: Vec<u8>,
    pub canonical_stacks_tip_consensus_hash: Vec<u8>,
    pub pox_valid: bool,
    pub accumulated_coinbase_ustx: i64,
    pub pox_payouts: Vec<u8>,
}

impl TryFrom<crate::types::Snapshot> for Snapshot {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: crate::types::Snapshot) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(Self {
            environment_id: value.environment_id,
            block_height: value.block_height as i32,
            burn_header_hash: value.burn_header_hash.0.to_vec(),
            sortition_id: value.sortition_id.0.to_vec(),
            parent_sortition_id: value.parent_sortition_id.0.to_vec(),
            burn_header_timestamp: value.burn_header_timestamp as i64,
            parent_burn_header_hash: value.parent_burn_header_hash.0.to_vec(),
            consensus_hash: value.consensus_hash.0.to_vec(),
            ops_hash: value.ops_hash.0.to_vec(),
            total_burn: value.total_burn as i64,
            sortition: value.is_sortition,
            sortition_hash: value.sortition_hash.0.to_vec(),
            winning_block_txid: value.winning_block_txid.0.to_vec(),
            winning_stacks_block_hash: value.winning_stacks_block_hash.0.to_vec(),
            index_root: value.index_root.0.to_vec(),
            num_sortitions: value.num_sortitions as i32,
            stacks_block_accepted: value.was_stacks_block_accepted,
            stacks_block_height: value.stacks_block_height as i32,
            arrival_index: value.arrival_index as i32,
            canonical_stacks_tip_height: value.canonical_stacks_tip_height as i32,
            canonical_stacks_tip_hash: value.canonical_stacks_tip_hash.0.to_vec(),
            canonical_stacks_tip_consensus_hash: value
                .canonical_stacks_tip_consensus_hash
                .0
                .to_vec(),
            pox_valid: value.is_pox_valid,
            accumulated_coinbase_ustx: value.accumulated_coinbase_ustx as i64,
            pox_payouts: rmp_serde::encode::to_vec(&value.pox_payouts)?,
        })
    }
}

impl TryFrom<Snapshot> for crate::types::Snapshot {
    type Error = color_eyre::eyre::Error;

    fn try_from(value: Snapshot) -> Result<Self> {
        Ok(Self {
            environment_id: value.environment_id,
            block_height: value.block_height as u32,
            burn_header_hash: stacks::BurnchainHeaderHash::from_vec(&value.burn_header_hash)
                .ok_or(anyhow!(
                    "failed to convert burn header hash bytes to BurnchainHeaderHash"
                ))?,
            sortition_id: stacks::SortitionId::from_vec(&value.sortition_id).ok_or(anyhow!(
                "failed to convert sortition id bytes to SortitionId"
            ))?,
            parent_sortition_id: stacks::SortitionId::from_vec(&value.parent_sortition_id).ok_or(
                anyhow!("failed to convert parent sortition id bytes to SortitionId"),
            )?,
            burn_header_timestamp: value.burn_header_timestamp as u64,
            parent_burn_header_hash: stacks::BurnchainHeaderHash::from_vec(
                &value.parent_burn_header_hash,
            )
            .ok_or(anyhow!(
                "failed to convert parent burn header hash to BurnchainHeaderHash"
            ))?,
            consensus_hash: stacks::ConsensusHash::from_vec(&value.consensus_hash).ok_or(
                anyhow!("failed to convert consensus hash bytes to ConsensusHash"),
            )?,
            ops_hash: stacks::OpsHash::from_vec(&value.ops_hash)
                .ok_or(anyhow!("failed to convert ops hash to OpsHash"))?,
            total_burn: value.total_burn as u64,
            is_sortition: value.sortition,
            sortition_hash: stacks::SortitionHash::from_vec(&value.sortition_hash).ok_or(
                anyhow!("failed to convert sortition hash bytes to SortitionHash"),
            )?,
            winning_block_txid: stacks::Txid::from_vec(&value.winning_block_txid)
                .ok_or(anyhow!("failed to convert winning block txid to Txid"))?,
            winning_stacks_block_hash: stacks::BlockHeaderHash::from_vec(
                &value.winning_stacks_block_hash,
            )
            .ok_or(anyhow!(
                "failed to convert winning stacks block hash to BlockHeaderHash"
            ))?,
            index_root: stacks::TrieHash::from_vec(&value.index_root)
                .ok_or(anyhow!("failed to convert index root to TrieHash"))?,
            num_sortitions: value.num_sortitions as u32,
            was_stacks_block_accepted: value.stacks_block_accepted,
            stacks_block_height: value.stacks_block_height as u32,
            arrival_index: value.arrival_index as u32,
            canonical_stacks_tip_height: value.canonical_stacks_tip_height as u32,
            canonical_stacks_tip_hash: stacks::BlockHeaderHash::from_vec(
                &value.canonical_stacks_tip_hash,
            )
            .ok_or(anyhow!(
                "failed to convert canonical stacks tip hash to BlockHeaderHash"
            ))?,
            canonical_stacks_tip_consensus_hash: stacks::ConsensusHash::from_vec(
                &value.canonical_stacks_tip_consensus_hash,
            )
            .ok_or(anyhow!(
                "failed to convert canonical stacks tip consensus hash to ConsensusHash"
            ))?,
            accumulated_coinbase_ustx: value.accumulated_coinbase_ustx as u64,
            is_pox_valid: value.pox_valid,
            pox_payouts: rmp_serde::decode::from_slice(&value.pox_payouts)?,
        })
    }
}
