use std::cell::RefCell;

use clarity::vm::types::QualifiedContractIdentifier;
use diesel::{SqliteConnection, prelude::*, OptionalExtension, insert_into};
use color_eyre::{Result, eyre::bail};
use lz4_flex::compress_prepend_size;

use crate::{schema::appdb::*, model::app_db::{ContractExecution, ContractVarInstance, Block, Environment, Contract}};

pub struct AppDb {
    conn: RefCell<SqliteConnection>
}

impl AppDb {
    pub fn new(conn: SqliteConnection) -> Self {
        Self { conn: RefCell::new(conn) }
    }

    pub fn get_env(&self, name: &str) -> Result<Option<Environment>> {
        let result = environment::table
            .filter(environment::name.like(name))
            .first(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    pub fn new_env(&self, name: &str, runtime_id: i32) -> Result<Environment> {
        let result = insert_into(environment::table)
            .values((
                environment::runtime_id.eq(runtime_id),
                environment::name.eq(name)
            ))
            .get_result::<Environment>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn get_contract_id(&self, contract_identifier: &QualifiedContractIdentifier) -> Result<Option<i32>> {
        let result = contract::table
            .filter(contract::qualified_contract_id.eq(contract_identifier.to_string()))
            .select(contract::id)
            .first::<i32>(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    pub fn get_var_id(&self, contract_id: i32, key: &str) -> Result<Option<i32>> {
        let result = contract_var::table
            .filter(
                contract_var::key.eq(key)
                .and(contract_var::contract_id.eq(contract_id))
            )
            .select(contract_var::id)
            .first::<i32>(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    pub fn insert_var_instance(&self, contract_execution_id: i32, contract_var_id: i32, value: &[u8]) -> Result<ContractVarInstance> {
        let result = insert_into(contract_var_instance::table)
            .values((
                contract_var_instance::columns::contract_execution_id.eq(contract_execution_id),
                contract_var_instance::columns::contract_var_id.eq(contract_var_id),
                contract_var_instance::columns::value.eq(value)
            ))
            .get_result::<ContractVarInstance>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn insert_execution(&self, block_id: i32, transaction_id: &[u8], contract_id: i32) -> Result<ContractExecution> {
        let result = insert_into(contract_execution::table)
            .values((
                contract_execution::contract_id.eq(contract_id),
                contract_execution::block_id.eq(block_id),
                contract_execution::transaction_id.eq(transaction_id)
            ))
            .get_result::<ContractExecution>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn get_var_latest(&self, contract_id: i32, key: &str) -> Result<Option<Vec<u8>>> {
        let contract_var_id = self.get_var_id(contract_id, key)
            .expect("failed to find contract var id")
            .expect("failed to find contract var");

        let result = contract_var_instance::table
            .filter(contract_var_instance::contract_var_id.eq(contract_var_id))
            .select(contract_var_instance::value)
            .order_by(contract_var_instance::id.desc())
            .first(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    pub fn insert_block(&self, environment_id: i32, height: i32, hash: &[u8], marf_trie_root_hash: &[u8]) -> Result<Block> {
        let result = insert_into(block::table)
            .values((
                block::height.eq(height),
                block::index_hash.eq(hash),
                block::environment_id.eq(environment_id),
                block::marf_trie_root_hash.eq(marf_trie_root_hash)
            ))
            .get_result::<Block>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn insert_environment(&self, runtime_id: i32, name: &str) -> Result<Environment> {
        let result = insert_into(environment::table)
            .values((
                environment::runtime_id.eq(runtime_id),
                environment::name.eq(name)
            ))
            .get_result::<Environment>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn insert_contract(&self, block_id: i32, contract_id: &str, source: &str) -> Result<Contract> {
        let compressed_source = compress_prepend_size(source.as_bytes());

        let result = insert_into(contract::table)
            .values((
                contract::qualified_contract_id.eq(contract_id.to_string()),
                contract::block_id.eq(block_id),
                contract::source.eq(&compressed_source)
            ))
            .on_conflict(contract::qualified_contract_id)
            .do_update()
            .set((
                contract::block_id.eq(block_id),
                contract::source.eq(&compressed_source)))
            .get_result::<Contract>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }
}