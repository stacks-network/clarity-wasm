use std::cell::RefCell;

use color_eyre::Result;
use diesel::prelude::*;
use diesel::{OptionalExtension, QueryDsl, SqliteConnection};

use crate::{clarity, stacks};

use super::model::sortition::{AstRuleHeight, Epoch};
use super::schema::sortition::*;

pub struct StacksBurnStateDb {
    conn: RefCell<SqliteConnection>,
    pox_constants: stacks::PoxConstants
}

impl StacksBurnStateDb {
    pub fn new(
        sortition_db_path: &str, 
        pox_constants: stacks::PoxConstants
    ) -> Result<Self> {
        Ok(Self {
            conn: RefCell::new(SqliteConnection::establish(sortition_db_path)?),
            pox_constants
        })
    }
}

impl clarity::BurnStateDB for StacksBurnStateDb {
    fn get_v1_unlock_height(&self) -> u32 {
        self.pox_constants.v1_unlock_height
    }

    fn get_v2_unlock_height(&self) -> u32 {
        self.pox_constants.v2_unlock_height
    }

    fn get_pox_3_activation_height(&self) -> u32 {
        self.pox_constants.pox_3_activation_height
    }

    fn get_burn_block_height(&self, sortition_id: &stacks::SortitionId) -> Option<u32> {
        todo!()
    }

    fn get_burn_start_height(&self) -> u32 {
        todo!()
    }

    fn get_pox_prepare_length(&self) -> u32 {
        self.pox_constants.prepare_length
    }

    fn get_pox_reward_cycle_length(&self) -> u32 {
        self.pox_constants.reward_cycle_length
    }

    fn get_pox_rejection_fraction(&self) -> u64 {
        self.pox_constants.pox_rejection_fraction
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
        let epoch = epochs::table
            .filter(epochs::epoch_id.eq(*epoch_id as i32))
            .get_result::<Epoch>(&mut *self.conn.borrow_mut())
            .optional()
            .expect("failed to query database");

        epoch.map(|e| e.into())
    }

    fn get_ast_rules(&self, height: u32) -> clarity::ASTRules {
        let rules = ast_rule_heights::table
            .get_results::<AstRuleHeight>(&mut *self.conn.borrow_mut())
            .expect("failed to query database");

        assert!(!rules.is_empty());
        let mut last_rule = &rules[0];
        for rule in rules.iter() {
            if last_rule.block_height as u32 <= height && height < rule.block_height as u32 {
                return Into::<clarity::ASTRules>::into(rule.clone());
            }
            last_rule = rule;
        }

        Into::<clarity::ASTRules>::into(last_rule.clone())
    }

    fn get_pox_payout_addrs(
        &self,
        height: u32,
        sortition_id: &stacks::SortitionId,
    ) -> Option<(Vec<clarity::TupleData>, u128)> {
        todo!()
    }
}
