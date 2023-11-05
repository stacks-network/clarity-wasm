use std::cell::RefCell;

use color_eyre::{
    eyre::anyhow,
    Result,
};
use diesel::{insert_into, prelude::*, OptionalExtension, SqliteConnection};
use lz4_flex::compress_prepend_size;

use crate::{
    clarity,
    db::{
        model::app_db::{
            Block, BlockHeader, Contract, ContractExecution, ContractVarInstance, 
            Environment,
        },
        schema::appdb::*,
    },
    stacks,
};

pub struct AppDb {
    conn: RefCell<SqliteConnection>,
}

impl AppDb {
    pub fn new(conn: SqliteConnection) -> Self {
        Self {
            conn: RefCell::new(conn),
        }
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
                environment::name.eq(name),
            ))
            .get_result::<Environment>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn get_contract_id(
        &self,
        contract_identifier: &clarity::QualifiedContractIdentifier,
    ) -> Result<Option<i32>> {
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
                contract_var::key
                    .eq(key)
                    .and(contract_var::contract_id.eq(contract_id)),
            )
            .select(contract_var::id)
            .first::<i32>(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    pub fn insert_var_instance(
        &self,
        contract_execution_id: i32,
        contract_var_id: i32,
        value: &[u8],
    ) -> Result<ContractVarInstance> {
        let result = insert_into(contract_var_instance::table)
            .values((
                contract_var_instance::columns::contract_execution_id.eq(contract_execution_id),
                contract_var_instance::columns::contract_var_id.eq(contract_var_id),
                contract_var_instance::columns::value.eq(value),
            ))
            .get_result::<ContractVarInstance>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn insert_execution(
        &self,
        block_id: i32,
        transaction_id: &[u8],
        contract_id: i32,
    ) -> Result<ContractExecution> {
        let result = insert_into(contract_execution::table)
            .values((
                contract_execution::contract_id.eq(contract_id),
                contract_execution::block_id.eq(block_id),
                contract_execution::transaction_id.eq(transaction_id),
            ))
            .get_result::<ContractExecution>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn get_var_latest(&self, contract_id: i32, key: &str) -> Result<Option<Vec<u8>>> {
        let contract_var_id = self
            .get_var_id(contract_id, key)
            .expect("sql query execution failed")
            .expect("failed to find contract var");

        let result = contract_var_instance::table
            .filter(contract_var_instance::contract_var_id.eq(contract_var_id))
            .select(contract_var_instance::value)
            .order_by(contract_var_instance::id.desc())
            .first(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    pub fn insert_block(
        &self,
        environment_id: i32,
        height: i32,
        hash: &[u8],
        marf_trie_root_hash: &[u8],
    ) -> Result<Block> {
        let result = insert_into(block::table)
            .values((
                block::height.eq(height),
                block::index_hash.eq(hash),
                block::environment_id.eq(environment_id),
                block::marf_trie_root_hash.eq(marf_trie_root_hash),
            ))
            .get_result::<Block>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn insert_environment(&self, runtime_id: i32, name: &str) -> Result<Environment> {
        let result = insert_into(environment::table)
            .values((
                environment::runtime_id.eq(runtime_id),
                environment::name.eq(name),
            ))
            .get_result::<Environment>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    pub fn insert_contract(
        &self,
        block_id: i32,
        contract_id: &str,
        source: &str,
    ) -> Result<Contract> {
        let compressed_source = compress_prepend_size(source.as_bytes());

        let result = insert_into(contract::table)
            .values((
                contract::qualified_contract_id.eq(contract_id.to_string()),
                contract::block_id.eq(block_id),
                contract::source.eq(&compressed_source),
            ))
            .on_conflict(contract::qualified_contract_id)
            .do_update()
            .set((
                contract::block_id.eq(block_id),
                contract::source.eq(&compressed_source),
            ))
            .get_result::<Contract>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    /// Attempts to fetch a [BlockHeader] from the database using its
    /// [stacks::StacksBlockId]. If no records are found, this function will return
    /// [None], and will panic if the query fails to execute.
    fn get_block_header_by_stacks_block_id(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Result<Option<BlockHeader>> {
        let result = _block_headers::table
            .filter(_block_headers::index_block_hash.eq(id_bhh.as_bytes().to_vec()))
            .first::<BlockHeader>(&mut *self.conn.borrow_mut())
            .optional()
            .map_err(|e| anyhow!("sql query execution failed"))?;

        Ok(result)
    }
}

/// Implementation of Clarity's [clarity::HeadersDB] for the app datastore.
impl clarity::HeadersDB for AppDb {
    /// Retrieves the [stacks::BlockHeaderHash] for the Stacks block header with the
    /// given index block hash.
    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::BlockHeaderHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .and_then(|header| {
                Some(stacks::BlockHeaderHash(
                    header
                        .index_block_hash
                        .try_into()
                        .expect("failed to convert index block hash into a 32-byte array"),
                ))
            })
    }

    /// Retrieves the [stacks::BurnchainHeaderHash] for the Stacks block header
    /// with the given index block hash.
    fn get_burn_header_hash_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::BurnchainHeaderHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .and_then(|header| {
                Some(stacks::BurnchainHeaderHash(
                    header
                        .burn_header_hash
                        .try_into()
                        .expect("failed to convert burn header hash into a 32-byte array"),
                ))
            })
    }

    /// Retrieves the [stacks::ConsensusHash] for the Stacks block header with
    /// the given index block hash.
    fn get_consensus_hash_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::ConsensusHash> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .and_then(|header| {
                Some(stacks::ConsensusHash(
                    header
                        .consensus_hash
                        .try_into()
                        .expect("failed to convert consensus hash into a 20-byte array"),
                ))
            })
    }

    /// Retrieves the [stacks::VRFSeed] (proof) for the Stacks block header with
    /// the given index block hash.
    fn get_vrf_seed_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::VRFSeed> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .and_then(|header| {
                Some(stacks::VRFSeed(
                    header
                        .proof
                        .try_into()
                        .expect("failed to convert the VRF seed (proof) into a 32-byte array")
                ))
            })
    }

    /// Retrieves the burn block timestamp as a [u64] for the Stacks block header
    /// with the given index block hash.
    fn get_burn_block_time_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u64> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .and_then(|header| Some(header.burn_header_timestamp as u64))
    }

    /// Retrieves the block height of the associated burn bunrh as a [u32] for
    /// the Stacks block header with the given index block hash.
    fn get_burn_block_height_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u32> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .and_then(|header| Some(header.burn_header_height as u32))
    }

    fn get_miner_address(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::StacksAddress> {
        todo!()
    }

    fn get_burnchain_tokens_spent_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        todo!()
    }

    fn get_burnchain_tokens_spent_for_winning_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        todo!()
    }

    fn get_tokens_earned_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        todo!()
    }
}
