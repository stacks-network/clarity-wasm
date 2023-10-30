use clarity::vm::database::ClarityBackingStore;

use sha2::{Sha512_256, Digest};
use stacks_common::{types::chainstate::StacksBlockId, util::hash::Sha512Trunc256Sum};

use crate::appdb::AppDb;
use crate::model::app_db::ContractExecution;



pub struct DataStore<'a> {
    exec: Option<ContractExecution>,
    db: &'a AppDb,
    open_chain_tip: StacksBlockId,
    current_chain_tip: StacksBlockId,
    chain_height: u32
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

    pub fn as_clarity_store(&mut self) -> &mut dyn ClarityBackingStore {
        self
    }

    pub fn set_exec(&mut self, exec: ContractExecution) {
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

impl ClarityBackingStore for DataStore<'_> {
    fn put_all(&mut self, items: Vec<(String, String)>) {

        for (key, value) in items {
            let contract_var_id = self.db.get_var_id(self.exec.as_ref().expect("contract execution not set").contract_id, &key)
                .expect("failed to find contract var id")
                .expect("contract var id does not exist");

            self.db.insert_var_instance(self.exec.as_ref().expect("contract execution not set").id, contract_var_id, value.as_bytes());
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        let buff = self.db.get_var_latest(self.exec.as_ref().unwrap().contract_id, key)
            .expect("failed to retrieve latest var");

        if let Some(str) = buff {
            Some(String::from_utf8(str).expect("failed to convert var to string"))
        } else {
            None
        }
    }

    fn get_with_proof(&mut self, _key: &str) -> Option<(String, Vec<u8>)> {
        None
    }

    fn set_block_hash(&mut self, bhh: stacks_common::types::chainstate::StacksBlockId) -> clarity::vm::errors::InterpreterResult<stacks_common::types::chainstate::StacksBlockId> {
        let prior_tip = self.open_chain_tip;
        self.current_chain_tip = bhh;
        Ok(prior_tip)
    }

    fn get_block_at_height(&mut self, height: u32) -> Option<stacks_common::types::chainstate::StacksBlockId> {
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