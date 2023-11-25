use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use diesel::helper_types::{Limit, Offset};
use diesel::prelude::*;
use diesel::query_dsl::methods::{LimitDsl, LoadQuery, OffsetDsl};
use diesel::upsert::excluded;
use diesel::{debug_query, insert_into, OptionalExtension, QueryDsl, SqliteConnection};
use log::*;
use lz4_flex::compress_prepend_size;

use super::dbcursor::RecordCursor;
#[allow(unused_imports)]
use crate::{
    clarity,
    db::{
        dbcursor,
        model::app_db::{
            Block, BlockHeader, Contract, ContractExecution, ContractVarInstance, Environment,
            MaturedReward, Payment,
        },
        schema::appdb::*,
    },
    stacks::{
        self,
        Address, // This import will give warnings but is needed for its impl fn's.
    },
};

pub struct AppDbBatchContext<'a> {
    conn: RefMut<'a, SqliteConnection>
}

impl<'a> AppDbBatchContext<'a> {
    pub fn new(conn: RefMut<'a, SqliteConnection>) -> Self {
        Self { conn }
    }

    pub fn import_snapshots(
        &mut self, 
        snapshots: Box<dyn Iterator<Item = Result<crate::types::Snapshot>>>
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for snapshot in snapshots {
                let snapshot = snapshot?;
    
                trace!(
                    "inserting snapshot {{sortition_id: {:?}, index_root: {:?}}}",
                    &snapshot.sortition_id,
                    &snapshot.index_root
                );
                let snapshot: super::model::app_db::Snapshot = snapshot.try_into()?;
    
                let insert_stmt = insert_into(_snapshots::table)
                    .values(snapshot)
                    .on_conflict(_snapshots::index_root)
                    .do_update()
                    .set(_snapshots::index_root.eq(excluded(_snapshots::index_root)));
    
                trace!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt));
    
                let affected_rows = insert_stmt.execute(tx)?;
    
                if affected_rows != 1 {
                    bail!("expected insert of one snapshot, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    pub fn import_block_commits(
        &mut self,
        block_commits: Box<dyn Iterator<Item = Result<crate::types::BlockCommit>>>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for block_commit in block_commits {
                let block_commit = block_commit?;

                trace!("inserting block commit {{txid: {:?}, sortition_id: {:?}", &block_commit.txid, &block_commit.sortition_id);
                let block_commit: super::model::app_db::BlockCommit = block_commit.try_into()?;

                let insert_stmt = insert_into(_block_commits::table)
                    .values(block_commit)
                    .on_conflict((_block_commits::txid, _block_commits::sortition_id))
                    .do_update()
                    .set((
                        _block_commits::txid.eq(excluded(_block_commits::txid)),
                        _block_commits::sortition_id.eq(excluded(_block_commits::sortition_id)),
                    ));

                trace!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one block commit, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    pub fn import_ast_rules(
        &mut self,
        rules: Box<dyn Iterator<Item = Result<crate::types::AstRuleHeight>>>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for rule in rules {
                let rule = rule?;

                trace!("inserting AST rule/height {{ast_rule_id: {}, block_height: {}}}", rule.ast_rule_id, rule.block_height);
                let rule: super::model::app_db::AstRuleHeight = rule.try_into()?;

                let insert_stmt = insert_into(_ast_rule_heights::table)
                    .values(rule)
                    .on_conflict(_ast_rule_heights::ast_rule_id)
                    .do_update()
                    .set(_ast_rule_heights::ast_rule_id.eq(excluded(_ast_rule_heights::ast_rule_id)));

                trace!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one AST rule height, but got {affected_rows} affected rows");
                }
            }

            ok!()
        })
    }

    pub fn import_epochs(
        &mut self,
        epochs: Box<dyn Iterator<Item = Result<crate::types::Epoch>>>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for epoch in epochs {
                let epoch = epoch?;

                trace!("inserting epoch {{epoch_id: {}, start_block: {}, end_block: {}}}", 
                    epoch.epoch_id, epoch.start_block_height, epoch.end_block_height);
                let epoch: super::model::app_db::Epoch = epoch.try_into()?;

                let insert_stmt = insert_into(_epochs::table)
                    .values(epoch)
                    .on_conflict((_epochs::start_block_height, _epochs::epoch_id))
                    .do_update()
                    .set((
                        _epochs::start_block_height.eq(excluded(_epochs::start_block_height)),
                        _epochs::epoch_id.eq(excluded(_epochs::epoch_id))
                    ));

                trace!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt));

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one epoch, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }
}

/// The application database API. Also used to implement instrumented Clarity
/// stores.
pub struct AppDb {
    conn: Rc<RefCell<SqliteConnection>>,
}

impl AppDb {
    /// Creates a new instance of [AppDb] using the provided
    /// [diesel::SqliteConnection].
    pub fn new(conn: SqliteConnection) -> Self {
        Self {
            conn: Rc::new(RefCell::new(conn)),
        }
    }

    pub fn batch(&self) -> AppDbBatchContext {
        AppDbBatchContext::new(self.conn.borrow_mut())
    }

    /// Returns a streaming iterator over the provided query. The `buffer_size_hint`
    /// specifies the maximum number of records which will be pre-fetched in each page.
    pub fn stream_results<Record, Model, Query>(
        &self,
        query: Query,
        buffer_size_hint: usize,
    ) -> impl Iterator<Item = Result<Model>>
    where
        Record: TryInto<Model>,
        Model: Clone,
        Query: OffsetDsl + Clone,
        Offset<Query>: LimitDsl,
        Limit<Offset<Query>>: for<'a> LoadQuery<'a, diesel::SqliteConnection, Record>,
    {
        Self::inner_stream_results(query, self.conn.clone(), buffer_size_hint)
    }

    pub fn stream_snapshots(&self) -> impl Iterator<Item = Result<crate::types::Snapshot>> {
        Self::inner_stream_results::<super::model::app_db::Snapshot, crate::types::Snapshot, _, _>(
            _snapshots::table,
            self.conn.clone(),
            1000,
        )
    }

    /// Get an object that implements the iterator interface.
    fn inner_stream_results<Record, Model, Query, Conn>(
        query: Query,
        conn: Rc<RefCell<Conn>>,
        buffer_size_hint: usize,
    ) -> impl Iterator<Item = Result<Model>>
    where
        Record: TryInto<Model>,
        Model: Clone,
        Query: OffsetDsl + Clone,
        Offset<Query>: LimitDsl,
        Limit<Offset<Query>>: for<'a> LoadQuery<'a, Conn, Record>,
    {
        RecordCursor {
            conn,
            query,
            cursor: 0,
            buffer: VecDeque::with_capacity(buffer_size_hint),
            record_type: PhantomData,
            model_type: PhantomData,
        }
    }

    /// Retrieves an existing runtime environment by name. Returns [None] if
    /// the environment was not found.
    pub fn get_env(&self, name: &str) -> Result<Option<Environment>> {
        let result = environment::table
            .filter(environment::name.like(name))
            .first(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    /// Retrieves a list over all [Environment]s existing in the local application
    /// database.
    pub fn list_envs(&self) -> Result<Vec<Environment>> {
        let results = environment::table
            .order_by(environment::id.asc())
            .get_results::<Environment>(&mut *self.conn.borrow_mut())?;

        Ok(results)
    }

    /// Retrieves the internal id of a contract using the specified
    /// [clarity::QualifiedContractIdentifier].
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

    /// Retrieves the internal id of a data-var for the specified contract.
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

    /// Inserts a Clarity data-var instance into the application database. A data-var
    /// instance is a data-var which is coupled to a specific contract execution,
    /// allowing for following changes to the variable through all contract
    /// executions throughout its lifetime.
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

    /// Inserts a new [ContractExecution] into the application database. Contract
    /// executions are a unique application-specific tracking of unique executions
    /// of a Clarity contract are used to relate contract sub-entities to the
    /// specific execution of a contract.
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

    /// Retrieves the latest instance of a Clarity contract's data-var by
    /// the internal contract id and the data-var's key.
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

    /// Inserts a new runtime environment into the application database. Runtime
    /// environments are used to distinguish different environments (configurations)
    /// from eachother for comparison purposes.
    pub fn insert_environment(
        &self,
        runtime_id: i32,
        name: &str,
        path: &str,
    ) -> Result<Environment> {
        let result = insert_into(environment::table)
            .values((
                environment::runtime_id.eq(runtime_id),
                environment::name.eq(name),
                environment::path.eq(path),
            ))
            .get_result::<Environment>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    /// Inserts a Clarity contract into the application-specific database and
    /// returns the resulting entity.
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
            .map_err(|e| anyhow!("sql query execution failed: {e:?}"))?;

        Ok(result)
    }

    /// Attempts to fetch a [Payment] from the database using its [stacks::StacksBlockId].
    /// If no records are found, this function will return [None], and will panic
    /// if the query fails to execute.
    fn get_payment_by_stacks_block_id(
        &self,
        id_bhh: &stacks::StacksBlockId,
    ) -> Result<Option<Payment>> {
        let result = _payments::table
            .filter(_payments::block_hash.eq(id_bhh.as_bytes().to_vec()))
            .first::<Payment>(&mut *self.conn.borrow_mut())
            .optional()
            .map_err(|e| anyhow!("sql query execution failed: {e:?}"))?;

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
            .map(|header| {
                stacks::BlockHeaderHash(
                    header
                        .index_block_hash
                        .try_into()
                        .expect("failed to convert index block hash into a 32-byte array"),
                )
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
            .map(|header| {
                stacks::BurnchainHeaderHash(
                    header
                        .burn_header_hash
                        .try_into()
                        .expect("failed to convert burn header hash into a 32-byte array"),
                )
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
            .map(|header| {
                stacks::ConsensusHash(
                    header
                        .consensus_hash
                        .try_into()
                        .expect("failed to convert consensus hash into a 20-byte array"),
                )
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
            .map(|header| {
                stacks::VRFSeed(
                    header
                        .proof
                        .try_into()
                        .expect("failed to convert the VRF seed (proof) into a 32-byte array"),
                )
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
            .map(|header| header.burn_header_timestamp as u64)
    }

    /// Retrieves the block height of the associated burn bunrh as a [u32] for
    /// the Stacks block header with the given index block hash.
    fn get_burn_block_height_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u32> {
        self.get_block_header_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|header| header.burn_header_height as u32)
    }

    /// Attempts to retrieve the [stacks::StacksAddress] of the miner who mined
    /// the specified [stacks::StacksBlockId]. Returns [None] if a [Payment] entry
    /// for the specified block could not be found.
    fn get_miner_address(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<stacks_common::types::chainstate::StacksAddress> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| {
                stacks::Address::from_string(&payment.address)
                    .expect("failed to convert the payment address to a StacksAddress")
            })
    }

    /// Attempts to retrieve the number of burnchain tokens spent for mining the
    /// specified [stacks::StacksBlockId] (`payments.burnchain_sortition_burn`).
    /// Returns [None] if a [Payment] entry for the specified block could not be found.
    /// TODO: Ensure that this description is correct.
    fn get_burnchain_tokens_spent_for_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| payment.burnchain_sortition_burn as u128)
    }

    /// Attempts to retrieve the number of burnchain tokens (i.e. BTC) spent for
    /// winning the specified [stacks::StacksBlockId] (`payments.burnchain_commit_burn`).
    /// Returns [None] if a [Payment] entry for the specified block could not be found.
    /// TODO: Ensure that this description is correct.
    fn get_burnchain_tokens_spent_for_winning_block(
        &self,
        id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        self.get_payment_by_stacks_block_id(id_bhh)
            .unwrap()
            .map(|payment| payment.burnchain_commit_burn as u128)
    }

    /// Attempts to retrieve the number of tokens (STX) which were earned for
    /// the specified [stacks::StacksBlockId].
    /// TODO: This method currently panics if
    /// anything isn't correct - this could be improved.
    /// TODO: Ensure that this description is correct.
    fn get_tokens_earned_for_block(
        &self,
        child_id_bhh: &stacks_common::types::chainstate::StacksBlockId,
    ) -> Option<u128> {
        let parent_id_bhh = self
            .get_block_header_by_stacks_block_id(child_id_bhh)
            .unwrap()
            .map(|header| header.parent_block_id)
            .expect("failed to find parent block header for child block");

        let rewards = _matured_rewards::table
            .filter(
                _matured_rewards::parent_index_block_hash
                    .eq(parent_id_bhh)
                    .and(
                        _matured_rewards::child_index_block_hash
                            .eq(child_id_bhh.as_bytes().to_vec()),
                    ),
            )
            .get_results::<MaturedReward>(&mut *self.conn.borrow_mut())
            .expect("failed to find matured rewards for parent+child block combination")
            .iter()
            .map(|result| result.into())
            .collect::<Vec<stacks::MinerReward>>();

        let reward = if rewards.len() == 2 {
            Some(if rewards[0].is_child() {
                rewards[0]
                    .try_add_parent(&rewards[1])
                    .expect("FATAL: got two child rewards")
            } else if rewards[1].is_child() {
                rewards[1]
                    .try_add_parent(&rewards[0])
                    .expect("FATAL: got two child rewards")
            } else {
                panic!("FATAL: got two parent rewards");
            })
        } else if child_id_bhh
            == &stacks::StacksBlockHeader::make_index_block_hash(
                &stacks::FIRST_BURNCHAIN_CONSENSUS_HASH,
                &stacks::FIRST_STACKS_BLOCK_HASH,
            )
        {
            Some(stacks::MinerReward::genesis(true)) //TODO: get this value from env
        } else {
            None
        };

        reward.and_then(|reward| reward.total().into())
    }
}
