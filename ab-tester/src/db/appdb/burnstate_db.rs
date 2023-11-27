use std::ops::Deref;
use std::rc::Rc;

use color_eyre::Result;
use diesel::prelude::*;
use diesel::{OptionalExtension, QueryDsl};

use super::AppDb;
use crate::db::model::app_db::*;
use crate::db::schema::appdb::*;
use crate::{clarity, stacks};

pub trait AsBurnStateDb {
    fn as_burnstate_db(&self) -> Result<&dyn clarity::BurnStateDB>;
}

pub struct AppDbBurnStateWrapper {
    environment_id: i32,
    app_db: Rc<AppDb>,
    pox_constants: stacks::PoxConstants,
}

impl AsBurnStateDb for AppDbBurnStateWrapper {
    fn as_burnstate_db(&self) -> Result<&dyn clarity::BurnStateDB> {
        Ok(self as &dyn clarity::BurnStateDB)
    }
}

impl AppDbBurnStateWrapper {
    pub fn new(
        environment_id: i32,
        app_db: Rc<AppDb>,
        pox_constants: stacks::PoxConstants,
    ) -> Self {
        Self {
            environment_id,
            app_db,
            pox_constants,
        }
    }
}

impl clarity::BurnStateDB for AppDbBurnStateWrapper {
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
        _snapshots::table
            .filter(
                _snapshots::environment_id
                    .eq(self.environment_id)
                    .and(_snapshots::sortition_id.eq(sortition_id.0.to_vec())),
            )
            .select(_snapshots::block_height)
            .get_result(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|height: i32| height as u32)
    }

    fn get_burn_start_height(&self) -> u32 {
        let first_height: i32 = _snapshots::table
            .filter(_snapshots::environment_id.eq(self.environment_id))
            .order_by(_snapshots::block_height.asc())
            .select(_snapshots::block_height)
            .get_result(&mut *self.app_db.conn.borrow_mut())
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
        _snapshots::table
            .filter(
                _snapshots::environment_id
                    .eq(self.environment_id)
                    .and(_snapshots::sortition_id.eq(sortition_id.0.to_vec()))
                    .and(_snapshots::block_height.eq(height as i32)),
            )
            .select(_snapshots::burn_header_hash)
            .get_result(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|hash: Vec<u8>| {
                stacks::BurnchainHeaderHash(
                    hash.try_into()
                        .expect("failed to convert burnchain header hash into a 32-byte array"),
                )
            })
    }

    fn get_sortition_id_from_consensus_hash(
        &self,
        consensus_hash: &stacks::ConsensusHash,
    ) -> Option<stacks::SortitionId> {
        _snapshots::table
            .filter(
                _snapshots::environment_id
                    .eq(self.environment_id)
                    .and(_snapshots::consensus_hash.eq(&consensus_hash.0.to_vec())),
            )
            .select(_snapshots::sortition_id)
            .get_result(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|bytes: Vec<u8>| {
                stacks::SortitionId(
                    bytes
                        .try_into()
                        .expect("failed to convert sortition id into 32-byte array"),
                )
            })
    }

    fn get_stacks_epoch(&self, height: u32) -> Option<clarity::StacksEpoch> {
        _epochs::table
            .filter(
                _epochs::start_block_height
                    .le(height as i32)
                    .and(_epochs::end_block_height.gt(height as i32)),
            )
            .get_result(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .expect("failed to execute database query")
            .map(|epoch: Epoch| {
                epoch
                    .try_into()
                    .expect("failed to convert epoch into clarity StacksEpoch")
            })
    }

    fn get_stacks_epoch_by_epoch_id(
        &self,
        epoch_id: &stacks::StacksEpochId,
    ) -> Option<clarity::StacksEpoch> {
        let epoch = _epochs::table
            .filter(_epochs::epoch_id.eq(*epoch_id as i32))
            .get_result::<Epoch>(&mut *self.app_db.conn.borrow_mut())
            .optional()
            .expect("failed to query database");

        epoch.map(|e| {
            e.try_into()
                .expect("failed to convert epoch into clarity StacksEpoch")
        })
    }

    fn get_ast_rules(&self, height: u32) -> clarity::ASTRules {
        let rules = _ast_rule_heights::table
            .get_results::<AstRuleHeight>(&mut *self.app_db.conn.borrow_mut())
            .expect("failed to query database");

        assert!(!rules.is_empty());
        let mut last_rule = &rules[0];
        for rule in rules.iter() {
            if last_rule.block_height as u32 <= height && height < rule.block_height as u32 {
                return rule
                    .clone()
                    .try_into()
                    .expect("failed to convert ast rule into Clarity ASTRules");
            }
            last_rule = rule;
        }

        last_rule
            .clone()
            .try_into()
            .expect("failed to convert last ast rule into Clarity ASTRules")
    }

    fn get_pox_payout_addrs(
        &self,
        _height: u32,
        _sortition_id: &stacks::SortitionId,
    ) -> Option<(Vec<clarity::TupleData>, u128)> {
        // This method is a bit involved, so hoping this doesn't need to be
        // implemented... we'll see :)
        todo!()
    }
}
