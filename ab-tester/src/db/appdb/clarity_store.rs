use std::rc::Rc;

use color_eyre::eyre::{anyhow, Result};
use sha2::{Digest, Sha512_256};

use super::AppDb;
use crate::{clarity, stacks};

pub struct AppDbContractExecutionContext {
    environment_id: i32,
    app_db: Rc<AppDb>,
    contract_id: i32,
    contract_execution_id: i32,
    current_chain_tip: stacks::StacksBlockId,
    open_chain_tip: stacks::StacksBlockId,
}

impl AppDbContractExecutionContext {
    pub fn new_contract_execution(
        app_db: Rc<AppDb>,
        environment_id: i32,
        block_id: i32,
        transaction_id: &[u8],
        contract_id: clarity::QualifiedContractIdentifier,
    ) -> Result<Self> {
        let inner_contract_id = app_db.get_contract_id(&contract_id)?.ok_or(anyhow!(
            "failed to fetch contract from app datastore with id '{contract_id}'"
        ))?;

        let contract_execution =
            app_db.insert_execution(block_id, transaction_id, inner_contract_id)?;

        Ok(Self {
            environment_id,
            app_db,
            contract_id: inner_contract_id,
            contract_execution_id: contract_execution.id,
            current_chain_tip: Self::height_to_id(0),
            open_chain_tip: Self::height_to_id(0),
        })
    }

    fn height_to_hashed_bytes(height: u32) -> [u8; 32] {
        let input_bytes = height.to_be_bytes();
        let mut hasher = Sha512_256::new();
        hasher.update(input_bytes);
        let hash = stacks::Sha512Trunc256Sum::from_hasher(hasher);
        hash.0
    }

    fn height_to_id(height: u32) -> stacks::StacksBlockId {
        stacks::StacksBlockId(Self::height_to_hashed_bytes(height))
    }
}

impl clarity::ClarityBackingStore for AppDbContractExecutionContext {
    fn put_all(&mut self, items: Vec<(String, String)>) {
        for (key, value) in items {
            let contract_var_id = self
                .app_db
                .get_var_id(self.contract_id, &key)
                .expect("failed to find contract var id")
                .expect("contract var id does not exist");

            self.app_db
                .insert_var_instance(
                    self.contract_execution_id,
                    contract_var_id,
                    value.as_bytes(),
                )
                .expect("failed to insert variable instance");
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        let buff = self
            .app_db
            .get_var_latest(self.contract_id, key)
            .expect("failed to retrieve latest var");

        buff.map(|str| String::from_utf8(str).expect("failed to convert var to string"))
    }

    fn get_with_proof(&mut self, key: &str) -> Option<(String, Vec<u8>)> {
        None
    }

    fn set_block_hash(
        &mut self,
        bhh: stacks::StacksBlockId,
    ) -> clarity::InterpreterResult<stacks::StacksBlockId> {
        let prior_tip = self.open_chain_tip;
        self.current_chain_tip = bhh;
        Ok(prior_tip)
    }

    fn get_block_at_height(&mut self, height: u32) -> Option<stacks::StacksBlockId> {
        todo!()
    }

    fn get_current_block_height(&mut self) -> u32 {
        todo!()
    }

    fn get_open_chain_tip_height(&mut self) -> u32 {
        todo!()
    }

    fn get_open_chain_tip(&mut self) -> stacks::StacksBlockId {
        todo!()
    }

    fn get_side_store(&mut self) -> &rusqlite::Connection {
        unimplemented!("Side-store not supported.");
    }
}
