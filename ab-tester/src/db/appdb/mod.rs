pub mod burnstate_db;
pub mod clarity_store;
pub mod headers_db;

use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use diesel::helper_types::{Limit, Offset};
use diesel::{prelude::*, update};
use diesel::query_dsl::methods::{LimitDsl, LoadQuery, OffsetDsl};
use diesel::upsert::excluded;
use diesel::{debug_query, insert_into, OptionalExtension, QueryDsl, SqliteConnection};
use log::*;
use lz4_flex::compress_prepend_size;

use super::dbcursor::RecordCursor;
use super::model::{
    Block, BlockHeader, Contract, ContractExecution, ContractVarInstance, Environment, Payment,
};
use super::schema::*;
#[allow(unused_imports)]
use crate::{
    clarity,
    db::appdb,
    stacks::{
        self,
        Address, // This import will give warnings but is needed for its impl fn's.
    },
};

/// Helper struct which encompasses batch operations for [AppDb].
pub struct AppDbBatchContext<'a> {
    conn: RefMut<'a, SqliteConnection>,
}

impl<'a> AppDbBatchContext<'a> {
    pub fn new(conn: RefMut<'a, SqliteConnection>) -> Self {
        Self { conn }
    }

    pub fn import_payments(
        &mut self,
        payments: Box<dyn Iterator<Item = Result<crate::types::Payment>>>,
        environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for payment in payments {
                let payment = payment
                    .map_err(|e| error!("{:?}", e))
                    .expect("failed to load payment");

                trace!("inserting payment {{address:  {:?}, block hash: {:?}}}", 
                    &payment.address,
                    &payment.block_hash);

                let mut payment: super::model::Payment = payment.try_into()?;

                if let Some(id) = environment_id {
                    payment.environment_id = id;
                }

                let insert_stmt = insert_into(_payments::table)
                    .values(payment)
                    .on_conflict((
                        _payments::environment_id,
                        _payments::address,
                        _payments::block_hash
                    ))
                    .do_update()
                    .set((
                        _payments::environment_id.eq(excluded(_payments::environment_id)),
                        _payments::address.eq(excluded(_payments::address)),
                        _payments::block_hash.eq(excluded(_payments::block_hash))
                    ));

                trace_sql!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one payment, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    pub fn import_block_headers(
        &mut self,
        headers: Box<dyn Iterator<Item = Result<crate::types::BlockHeader>>>,
        environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for header in headers {
                let header = header
                    .map_err(|e| error!("{:?}", e))
                    .expect("failed to load header");

                trace!(
                    "inserting block header {{hash: {:?}, height: {:?}}}",
                    &header.block_hash,
                    &header.block_height
                );
                let mut header: super::model::BlockHeader = header.try_into()?;

                if let Some(id) = environment_id {
                    header.environment_id = id;
                }

                let insert_stmt = insert_into(_block_headers::table)
                    .values(header)
                    .on_conflict((
                        //environment_id, consensus_hash, block_hash
                        _block_headers::environment_id,
                        _block_headers::consensus_hash,
                        _block_headers::block_hash
                    ))
                    .do_update()
                    .set((
                        _block_headers::environment_id.eq(excluded(_block_headers::environment_id)),
                        _block_headers::consensus_hash.eq(excluded(_block_headers::consensus_hash)),
                        _block_headers::block_hash.eq(excluded(_block_headers::block_hash))
                    ));

                trace_sql!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one block header, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    /// Imports snapshots from the provided source iterator into the app's datastore,
    /// optionally associating the entries with the provided `environment_id`. The
    /// import is performed in a single transaction, meaning any failures will roll-back
    /// all changes.
    pub fn import_snapshots(
        &mut self,
        snapshots: Box<dyn Iterator<Item = Result<crate::types::Snapshot>>>,
        environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for snapshot in snapshots {
                let snapshot = snapshot?;

                trace_sql!(
                    "inserting snapshot {{sortition_id: {:?}, index_root: {:?}}}",
                    &snapshot.sortition_id,
                    &snapshot.index_root
                );
                let mut snapshot: super::model::Snapshot = snapshot.try_into()?;

                if let Some(id) = environment_id {
                    snapshot.environment_id = id;
                }

                let insert_stmt = insert_into(_snapshots::table)
                    .values(snapshot)
                    .on_conflict((_snapshots::environment_id, _snapshots::index_root))
                    .do_update()
                    .set((
                        _snapshots::environment_id.eq(excluded(_snapshots::environment_id)),
                        _snapshots::index_root.eq(excluded(_snapshots::index_root)),
                    ));

                trace_sql!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one snapshot, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }

    /// Imports block commits from the provided source iterator into the app's
    /// datastore, optionally associating each entry with the provided `environment_id`.
    /// The import is run in a single database transaction, meaning that any failures
    /// will roll-back all changes.
    pub fn import_block_commits(
        &mut self,
        block_commits: Box<dyn Iterator<Item = Result<crate::types::BlockCommit>>>,
        environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for block_commit in block_commits {
                let block_commit = block_commit?;

                trace!("inserting block commit {{txid: {:?}, sortition_id: {:?}", &block_commit.txid, &block_commit.sortition_id);
                let mut block_commit: super::model::BlockCommit = block_commit.try_into()?;

                if let Some(id) = environment_id {
                    block_commit.environment_id = id;
                }

                let insert_stmt = insert_into(_block_commits::table)
                    .values(block_commit)
                    .on_conflict((
                        _block_commits::environment_id,
                        _block_commits::txid,
                        _block_commits::sortition_id
                    ))
                    .do_update()
                    .set((
                        _block_commits::environment_id.eq(excluded(_block_commits::environment_id)),
                        _block_commits::txid.eq(excluded(_block_commits::txid)),
                        _block_commits::sortition_id.eq(excluded(_block_commits::sortition_id)),
                    ));

                trace_sql!(
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

    /// Imports AST rules from the provided source iterator into the app's datastore,
    /// optionally associating each entry with the provided `environment_id`. The
    /// import is run in a single database transaction, meaning that any failures
    /// will roll-back all changes.
    pub fn import_ast_rules(
        &mut self,
        rules: Box<dyn Iterator<Item = Result<crate::types::AstRuleHeight>>>,
        environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for rule in rules {
                let rule = rule?;

                trace!("inserting AST rule/height {{ast_rule_id: {}, block_height: {}}}", rule.ast_rule_id, rule.block_height);
                let mut rule: super::model::AstRuleHeight = rule.try_into()?;

                if let Some(id) = environment_id {
                    rule.environment_id = id;
                }

                let insert_stmt = insert_into(_ast_rule_heights::table)
                    .values(rule)
                    .on_conflict((
                        _ast_rule_heights::environment_id,
                        _ast_rule_heights::ast_rule_id
                    ))
                    .do_update()
                    .set((
                        _ast_rule_heights::environment_id.eq(excluded(_ast_rule_heights::environment_id)),
                        _ast_rule_heights::ast_rule_id.eq(excluded(_ast_rule_heights::ast_rule_id))
                    ));

                trace_sql!(
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

    /// Import epochs from the provided source iterator into the app's datastore,
    /// optionally associating each entry with the provided `environment_id`. The
    /// import is run within a single database transaction, meaning that any failures
    /// will roll-back all changes.
    pub fn import_epochs(
        &mut self,
        epochs: Box<dyn Iterator<Item = Result<crate::types::Epoch>>>,
        environment_id: Option<i32>,
    ) -> Result<()> {
        let conn = &mut *self.conn;

        conn.transaction(|tx| -> Result<()> {
            for epoch in epochs {
                let epoch = epoch?;

                trace!(
                    "inserting epoch {{epoch_id: {}, start_block: {}, end_block: {}}}",
                    epoch.epoch_id,
                    epoch.start_block_height,
                    epoch.end_block_height
                );
                let mut epoch: super::model::Epoch = epoch.try_into()?;

                if let Some(id) = environment_id {
                    epoch.environment_id = id;
                }

                let insert_stmt = insert_into(_epochs::table)
                    .values(epoch)
                    .on_conflict((
                        _epochs::environment_id,
                        _epochs::start_block_height,
                        _epochs::epoch_id,
                    ))
                    .do_update()
                    .set((
                        _epochs::environment_id.eq(excluded(_epochs::environment_id)),
                        _epochs::start_block_height.eq(excluded(_epochs::start_block_height)),
                        _epochs::epoch_id.eq(excluded(_epochs::epoch_id)),
                    ));

                trace_sql!(
                    "SQL: {}",
                    debug_query::<diesel::sqlite::Sqlite, _>(&insert_stmt)
                );

                let affected_rows = insert_stmt.execute(tx)?;

                if affected_rows != 1 {
                    bail!("expected insert of one epoch, but got {affected_rows} affected rows");
                }
            }
            ok!()
        })
    }
}

/// The application database API. The [AppDb] stores application state, and is
/// also used to provide instrumented datastores for Clarity and Chainstate
/// operations on a per-environment basis.
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
        <Record as TryInto<Model>>::Error: std::fmt::Debug,
        Model: Clone + TryFrom<Record>,
        Query: OffsetDsl + Clone,
        Offset<Query>: LimitDsl,
        Limit<Offset<Query>>: for<'a> LoadQuery<'a, diesel::SqliteConnection, Record>,
    {
        Self::inner_stream_results(query, self.conn.clone(), buffer_size_hint)
    }

    pub fn stream_snapshots(&self) -> impl Iterator<Item = Result<crate::types::Snapshot>> {
        Self::inner_stream_results::<super::model::Snapshot, crate::types::Snapshot, _, _>(
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
        <Record as TryInto<Model>>::Error: std::fmt::Debug,
        Model: Clone + TryFrom<Record>,
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

    pub fn delete_blocks_for_environment(&self, environment_id: i32) -> Result<u32> {
        let result = diesel::delete(block::table)
            .filter(block::environment_id.eq(environment_id))
            .execute(&mut *self.conn.borrow_mut())?;

        Ok(result as u32)
    }

    pub fn snapshot_count(&self, environment_id: i32) -> Result<usize> {
        let result = _snapshots::table
            .filter(_snapshots::environment_id.eq(environment_id))
            .count()
            .get_result::<i64>(&mut *self.conn.borrow_mut())?;

        Ok(result as usize)
    }

    pub fn block_commit_count(&self, environment_id: i32) -> Result<usize> {
        let result = _block_commits::table
            .filter(_block_commits::environment_id.eq(environment_id))
            .count()
            .get_result::<i64>(&mut *self.conn.borrow_mut())?;

        Ok(result as usize)
    }

    pub fn ast_rule_count(&self, environment_id: i32) -> Result<usize> {
        let result = _ast_rule_heights::table
            .filter(_ast_rule_heights::environment_id.eq(environment_id))
            .count()
            .get_result::<i64>(&mut *self.conn.borrow_mut())?;

        Ok(result as usize)
    }

    pub fn epoch_count(&self, environment_id: i32) -> Result<usize> {
        let result = _epochs::table
            .filter(_epochs::environment_id.eq(environment_id))
            .count()
            .get_result::<i64>(&mut *self.conn.borrow_mut())?;

        Ok(result as usize)
    }

    pub fn payment_count(&self, environment_id: i32) -> Result<usize> {
        let result = _payments::table
            .filter(_payments::environment_id.eq(environment_id))
            .count()
            .get_result::<i64>(&mut *self.conn.borrow_mut())?;

        Ok(result as usize)
    }

    /// Retrieves an existing runtime environment by name. Returns [None] if
    /// the environment was not found.
    pub fn get_env(&self, name: &str) -> Result<Option<Environment>> {
        let query = environment::table.filter(environment::name.like(name));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query.first(&mut *self.conn.borrow_mut()).optional()?;

        Ok(result)
    }

    /// Retrieves a list over all [Environment]s existing in the local application
    /// database.
    pub fn list_envs(&self) -> Result<Vec<Environment>> {
        let query = environment::table.order_by(environment::id.asc());

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let results = query.get_results::<Environment>(&mut *self.conn.borrow_mut())?;

        Ok(results)
    }

    /// Retrieves the internal id of a contract using the specified
    /// [clarity::QualifiedContractIdentifier].
    pub fn get_contract_id(
        &self,
        contract_identifier: &clarity::QualifiedContractIdentifier,
    ) -> Result<Option<i32>> {
        let query = contract::table
            .filter(contract::qualified_contract_id.eq(contract_identifier.to_string()))
            .select(contract::id);

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
            .first::<i32>(&mut *self.conn.borrow_mut())
            .optional()?;

        Ok(result)
    }

    /// Retrieves the internal id of a data-var for the specified contract.
    pub fn get_var_id(&self, contract_id: i32, key: &str) -> Result<Option<i32>> {
        let query = contract_var::table
            .filter(
                contract_var::key
                    .eq(key)
                    .and(contract_var::contract_id.eq(contract_id)),
            )
            .select(contract_var::id);

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
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
        let query = insert_into(contract_var_instance::table).values((
            contract_var_instance::columns::contract_execution_id.eq(contract_execution_id),
            contract_var_instance::columns::contract_var_id.eq(contract_var_id),
            contract_var_instance::columns::value.eq(value),
        ));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
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
        let query = insert_into(contract_execution::table).values((
            contract_execution::contract_id.eq(contract_id),
            contract_execution::block_id.eq(block_id),
            contract_execution::transaction_id.eq(transaction_id),
        ));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
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

        let query = contract_var_instance::table
            .filter(contract_var_instance::contract_var_id.eq(contract_var_id))
            .select(contract_var_instance::value)
            .order_by(contract_var_instance::id.desc());

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query.first(&mut *self.conn.borrow_mut()).optional()?;

        Ok(result)
    }

    pub fn insert_block(
        &self,
        environment_id: i32,
        height: i32,
        hash: &[u8],
        marf_trie_root_hash: &[u8],
    ) -> Result<Block> {
        let query = insert_into(block::table).values((
            block::height.eq(height),
            block::index_hash.eq(hash),
            block::environment_id.eq(environment_id),
            block::marf_trie_root_hash.eq(marf_trie_root_hash),
        ));

        info!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
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
        let query = insert_into(environment::table).values((
            environment::runtime_id.eq(runtime_id),
            environment::name.eq(name),
            environment::base_path.eq(path),
        ));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
            .get_result::<Environment>(&mut *self.conn.borrow_mut())
            .unwrap();

        Ok(result)
    }

    /// Updates an environment's last-processed block height, which enables
    /// resume-processing functionality.
    pub fn set_environment_last_block_height(
        &self, 
        environment_id: i32, 
        last_block_height: i32
    ) -> Result<()> {
        update(environment::table)
            .filter(environment::id.eq(environment_id))
            .set(environment::last_block_height.eq(last_block_height))
            .execute(&mut *self.conn.borrow_mut())?;

        Ok(())
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

        let query = insert_into(contract::table)
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
            ));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
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
        let query = _block_headers::table
            .filter(_block_headers::index_block_hash.eq(id_bhh.as_bytes().to_vec()));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
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
        let query = _payments::table.filter(_payments::block_hash.eq(id_bhh.as_bytes().to_vec()));

        trace_sql!("SQL: {}", debug_query::<diesel::sqlite::Sqlite, _>(&query));

        let result = query
            .first::<Payment>(&mut *self.conn.borrow_mut())
            .optional()
            .map_err(|e| anyhow!("sql query execution failed: {e:?}"))?;

        Ok(result)
    }
}
