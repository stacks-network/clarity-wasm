use std::cell::RefCell;

use clarity::BurnStateDB;
use color_eyre::Result;
use diesel::prelude::*;
use diesel::{OptionalExtension, QueryDsl, SqliteConnection};

use super::model::sortition_db::{AstRuleHeight, Epoch};
use super::schema::sortition::*;
use crate::{clarity, stacks};

pub struct StacksBurnStateDb {
    conn: RefCell<SqliteConnection>,
    pox_constants: stacks::PoxConstants,
}

impl StacksBurnStateDb {
    pub fn new(sortition_db_path: &str, pox_constants: stacks::PoxConstants) -> Result<Self> {
        Ok(Self {
            conn: RefCell::new(SqliteConnection::establish(sortition_db_path)?),
            pox_constants,
        })
    }

    /// Gets the difference between `block_height` and the first burnchain
    /// block height. Returns [None] if the calculated value is negative.
    fn get_adjusted_block_height(&self, block_height: u32) -> Option<u32> {
        let first_block_height = self.get_burn_start_height();
        if block_height < first_block_height {
            return None;
        }
        Some(block_height - first_block_height)
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
        snapshots::table
            .filter(snapshots::sortition_id.eq(sortition_id.to_hex()))
            .select(snapshots::block_height)
            .get_result(&mut *self.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|height: i32| height as u32)
    }

    fn get_burn_start_height(&self) -> u32 {
        let first_height: i32 = snapshots::table
            .order_by(snapshots::block_height.asc())
            .select(snapshots::block_height)
            .get_result(&mut *self.conn.borrow_mut())
            .expect("failed to execute database query");

        first_height as u32
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
        snapshots::table
            .filter(
                snapshots::sortition_id
                    .eq(sortition_id.to_hex())
                    .and(snapshots::block_height.eq(height as i32)),
            )
            .select(snapshots::burn_header_hash)
            .get_result(&mut *self.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|hash: String| {
                stacks::BurnchainHeaderHash::from_hex(&hash)
                    .expect("failed to parse burnchain header hash hex")
            })
    }

    fn get_sortition_id_from_consensus_hash(
        &self,
        consensus_hash: &stacks::ConsensusHash,
    ) -> Option<stacks::SortitionId> {
        snapshots::table
            .filter(snapshots::consensus_hash.eq(&consensus_hash.to_hex()))
            .select(snapshots::sortition_id)
            .get_result(&mut *self.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|hex: String| {
                stacks::SortitionId::from_hex(&hex).expect("failed to parse sortition id hex")
            })
    }

    fn get_stacks_epoch(&self, height: u32) -> Option<clarity::StacksEpoch> {
        epochs::table
            .filter(
                epochs::start_block_height
                    .le(height as i32)
                    .and(epochs::end_block_height.gt(height as i32)),
            )
            .get_result(&mut *self.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|epoch: Epoch| epoch.into())
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
