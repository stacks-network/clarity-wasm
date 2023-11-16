use std::cell::RefCell;

use color_eyre::eyre::anyhow;
use color_eyre::Result;
use diesel::prelude::*;
use diesel::{OptionalExtension, QueryDsl, SqliteConnection};

use crate::{clarity, stacks};

pub struct StacksBurnStateDb {
    conn: RefCell<SqliteConnection>,
}

impl StacksBurnStateDb {
    pub fn new(sortition_db_path: &str) -> Result<Self> {
        Ok(Self {
            conn: RefCell::new(SqliteConnection::establish(sortition_db_path)?),
        })
    }
}

impl clarity::BurnStateDB for StacksBurnStateDb {
    fn get_v1_unlock_height(&self) -> u32 {
        todo!()
    }

    fn get_v2_unlock_height(&self) -> u32 {
        todo!()
    }

    fn get_pox_3_activation_height(&self) -> u32 {
        todo!()
    }

    fn get_burn_block_height(&self, sortition_id: &stacks::SortitionId) -> Option<u32> {
        todo!()
    }

    fn get_burn_start_height(&self) -> u32 {
        todo!()
    }

    fn get_pox_prepare_length(&self) -> u32 {
        todo!()
    }

    fn get_pox_reward_cycle_length(&self) -> u32 {
        todo!()
    }

    fn get_pox_rejection_fraction(&self) -> u64 {
        todo!()
    }

    fn get_burn_header_hash(
        &self,
        height: u32,
        sortition_id: &stacks::SortitionId,
    ) -> Option<stacks::BurnchainHeaderHash> {
        todo!()
    }

    fn get_sortition_id_from_consensus_hash(
        &self,
        consensus_hash: &stacks::ConsensusHash,
    ) -> Option<stacks::SortitionId> {
        todo!()
    }

    fn get_stacks_epoch(&self, height: u32) -> Option<clarity::StacksEpoch> {
        todo!()
    }

    fn get_stacks_epoch_by_epoch_id(
        &self,
        epoch_id: &stacks::StacksEpochId,
    ) -> Option<clarity::StacksEpoch> {
        todo!()
    }

    fn get_ast_rules(&self, height: u32) -> clarity::ASTRules {
        todo!()
    }

    fn get_pox_payout_addrs(
        &self,
        height: u32,
        sortition_id: &stacks::SortitionId,
    ) -> Option<(Vec<clarity::TupleData>, u128)> {
        todo!()
    }
}
