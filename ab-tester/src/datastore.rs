use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use sha2::{Digest, Sha512_256};
use stacks_common::{types::chainstate::StacksBlockId, util::hash::Sha512Trunc256Sum};

use crate::{
    appdb::AppDb,
    clarity, model,
    schema::{self, chainstate_marf::block_headers},
    stacks,
};

pub struct DataStore<'a> {
    exec: Option<model::app_db::ContractExecution>,
    db: &'a AppDb,
    open_chain_tip: StacksBlockId,
    current_chain_tip: StacksBlockId,
    chain_height: u32,
}

impl<'a> DataStore<'a> {
    pub fn new(db: &'a AppDb) -> Self {
        let id = Self::height_to_id(0);

        Self {
            exec: None,
            db,
            open_chain_tip: id,
            current_chain_tip: id,
            chain_height: 0,
        }
    }

    pub fn as_clarity_store(&mut self) -> &mut dyn clarity::ClarityBackingStore {
        self
    }

    pub fn set_exec(&mut self, exec: model::app_db::ContractExecution) {
        self.exec = Some(exec);
    }

    fn height_to_hashed_bytes(height: u32) -> [u8; 32] {
        let input_bytes = height.to_be_bytes();
        let mut hasher = Sha512_256::new();
        hasher.update(input_bytes);
        let hash = Sha512Trunc256Sum::from_hasher(hasher);
        hash.0
    }

    fn height_to_id(height: u32) -> StacksBlockId {
        StacksBlockId(Self::height_to_hashed_bytes(height))
    }
}

impl clarity::ClarityBackingStore for DataStore<'_> {
    fn put_all(&mut self, items: Vec<(String, String)>) {
        for (key, value) in items {
            let contract_var_id = self
                .db
                .get_var_id(
                    self.exec
                        .as_ref()
                        .expect("contract execution not set")
                        .contract_id,
                    &key,
                )
                .expect("failed to find contract var id")
                .expect("contract var id does not exist");

            self.db
                .insert_var_instance(
                    self.exec.as_ref().expect("contract execution not set").id,
                    contract_var_id,
                    value.as_bytes(),
                )
                .expect("failed to insert variable instance");
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        let buff = self
            .db
            .get_var_latest(self.exec.as_ref().unwrap().contract_id, key)
            .expect("failed to retrieve latest var");

        buff.map(|str| String::from_utf8(str).expect("failed to convert var to string"))
    }

    fn get_with_proof(&mut self, _key: &str) -> Option<(String, Vec<u8>)> {
        None
    }

    fn set_block_hash(
        &mut self,
        bhh: stacks_common::types::chainstate::StacksBlockId,
    ) -> clarity::InterpreterResult<stacks_common::types::chainstate::StacksBlockId> {
        let prior_tip = self.open_chain_tip;
        self.current_chain_tip = bhh;
        Ok(prior_tip)
    }

    fn get_block_at_height(
        &mut self,
        height: u32,
    ) -> Option<stacks_common::types::chainstate::StacksBlockId> {
        Some(Self::height_to_id(height))
    }

    fn get_current_block_height(&mut self) -> u32 {
        todo!()
    }

    fn get_open_chain_tip_height(&mut self) -> u32 {
        self.chain_height
    }

    fn get_open_chain_tip(&mut self) -> stacks_common::types::chainstate::StacksBlockId {
        self.open_chain_tip
    }

    fn get_side_store(&mut self) -> &rusqlite::Connection {
        unimplemented!("Side-store not supported.");
    }
}

impl clarity::BurnStateDB for DataStore<'_> {
    fn get_v1_unlock_height(&self) -> u32 {
        todo!()
    }

    fn get_v2_unlock_height(&self) -> u32 {
        todo!()
    }

    fn get_pox_3_activation_height(&self) -> u32 {
        todo!()
    }

    fn get_burn_block_height(
        &self,
        sortition_id: &stacks_common::types::chainstate::SortitionId,
    ) -> Option<u32> {
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

    fn get_stacks_epoch(&self, height: u32) -> Option<clarity::StacksEpoch> {
        todo!()
    }

    fn get_stacks_epoch_by_epoch_id(
        &self,
        epoch_id: &stacks_common::types::StacksEpochId,
    ) -> Option<clarity::StacksEpoch> {
        todo!()
    }

    fn get_ast_rules(&self, height: u32) -> clarity::ASTRules {
        clarity::ASTRules::PrecheckSize
    }

    fn get_pox_payout_addrs(
        &self,
        height: u32,
        sortition_id: &stacks_common::types::chainstate::SortitionId,
    ) -> Option<(Vec<clarity::TupleData>, u128)> {
        todo!()
    }
}
