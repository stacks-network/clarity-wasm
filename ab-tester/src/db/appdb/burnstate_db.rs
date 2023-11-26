use std::rc::Rc;

use crate::clarity;
use super::AppDb;

pub trait AsBurnStateDb {
    fn as_burnstate_db(&self) -> &dyn clarity::BurnStateDB;
}

pub struct AppDbBurnStateWrapper {
    environment_id: i32,
    app_db: Rc<AppDb>
}

impl AsBurnStateDb for AppDbBurnStateWrapper {
    fn as_burnstate_db(&self) -> &dyn clarity::BurnStateDB {
        self as &dyn clarity::BurnStateDB
    }
}

impl clarity::BurnStateDB for AppDbBurnStateWrapper {
    fn get_v1_unlock_height(&self) -> u32 {
        todo!()
    }

    fn get_v2_unlock_height(&self) -> u32 {
        todo!()
    }

    fn get_pox_3_activation_height(&self) -> u32 {
        todo!()
    }

    fn get_burn_block_height(&self, sortition_id: &stacks_common::types::chainstate::SortitionId) -> Option<u32> {
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
        sortition_id: &stacks_common::types::chainstate::SortitionId,
    ) -> Option<stacks_common::types::chainstate::BurnchainHeaderHash> {
        todo!()
    }

    fn get_sortition_id_from_consensus_hash(
        &self,
        consensus_hash: &stacks_common::types::chainstate::ConsensusHash,
    ) -> Option<stacks_common::types::chainstate::SortitionId> {
        todo!()
    }

    fn get_stacks_epoch(&self, height: u32) -> Option<::clarity::vm::StacksEpoch> {
        todo!()
    }

    fn get_stacks_epoch_by_epoch_id(&self, epoch_id: &stacks_common::types::StacksEpochId) -> Option<::clarity::vm::StacksEpoch> {
        todo!()
    }

    fn get_ast_rules(&self, height: u32) -> ::clarity::vm::ast::ASTRules {
        todo!()
    }

    fn get_pox_payout_addrs(
        &self,
        height: u32,
        sortition_id: &stacks_common::types::chainstate::SortitionId,
    ) -> Option<(Vec<::clarity::vm::types::TupleData>, u128)> {
        todo!()
    }
}