use std::cell::RefCell;

use color_eyre::{eyre::{anyhow, bail}, Result};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use crate::{
    context::{
        blocks::BlockHeader, boot_data::mainnet_boot_data, BlockCursor, Network, Runtime,
        TestEnvPaths, environments::BlockTransactionContext, Block
    },
    db::model::app_db as model,
    db::{schema::appdb, appdb::AppDb},
    ok, stacks,
};

use super::{ReadableEnv, RuntimeEnv, WriteableEnv};

/// Holds the configuration of an [InstrumentedEnv].
pub struct InstrumentedEnvConfig<'a> {
    working_dir: &'a str,
    readonly: bool,
    paths: TestEnvPaths,
    runtime: Runtime,
    network: Network,
}

/// Holds the opened state of an [InstrumentedEnv].
pub struct InstrumentedEnvState {
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    sortition_db: stacks::SortitionDB,
}

/// This environment type is app-specific and will instrument all Clarity-related
/// operations. This environment can be used for comparisons.
pub struct InstrumentedEnv<'a> {
    id: i32,
    name: &'a str,
    app_db: &'a AppDb,
    env_config: InstrumentedEnvConfig<'a>,
    env_state: Option<InstrumentedEnvState>,
}

impl<'a> InstrumentedEnv<'a> {
    /// Creates a new [InstrumentedEnv]. This method expects the provided
    /// `working_dir` to either be uninitialized or be using the same [Runtime]
    /// and [Network] configuration.
    pub fn new(
        id: i32,
        name: &'a str,
        app_db: &'a AppDb,
        working_dir: &'a str,
        runtime: Runtime,
        network: Network,
    ) -> Result<Self> {
        let paths = TestEnvPaths::new(working_dir);

        let env_config = InstrumentedEnvConfig {
            working_dir,
            readonly: false,
            paths,
            runtime,
            network,
        };

        Ok(Self {
            id,
            name,
            app_db,
            env_config,
            env_state: None,
        })
    }

    fn readonly(&mut self, readonly: bool) {
        self.env_config.readonly = readonly;
    }

    /// Attempts to retrieve the [InstrumentedEnvState] for this environment. Will
    /// return an error if [RuntimeEnv::open] has not been called.
    fn get_env_state(&self) -> Result<&InstrumentedEnvState> {
        let state = self.env_state
            .as_ref()
            .ok_or(anyhow!("[{}] environment has not been opened", self.name))?;

        Ok(state)
    }

    /// Attempts to retrieve the [InstrumentedEnvState] for this environment as a 
    /// mutable reference. Will return an error if [RuntimeEnv::open] has not been called.
    fn get_env_state_mut(&mut self) -> Result<&mut InstrumentedEnvState> {
        let state = self.env_state
            .as_mut()
            .ok_or(anyhow!("[{}] environment has not been opened", self.name))?;

        Ok(state)
    }

    /// Retrieve all block headers from the underlying storage.
    fn block_headers(&self) -> Result<Vec<BlockHeader>> {
        // Get our state
        let state = self
            .env_state
            .as_ref()
            .ok_or(anyhow!("environment has not been opened"))?;

        // Retrieve the tip.
        let tip = appdb::_block_headers::table
            .order_by(appdb::_block_headers::block_height.desc())
            .limit(1)
            .get_result::<model::BlockHeader>(&mut *state.index_db_conn.borrow_mut())?;

        let mut current_block = Some(tip);
        let mut headers: Vec<BlockHeader> = Vec::new();

        // Walk backwards
        while let Some(block) = current_block {
            let block_parent = appdb::_block_headers::table
                .filter(appdb::_block_headers::index_block_hash.eq(&block.parent_block_id))
                .get_result::<model::BlockHeader>(&mut *state.index_db_conn.borrow_mut())
                .optional()?;

            if let Some(parent) = &block_parent {
                headers.push(BlockHeader::new(
                    block.block_height as u32,
                    hex::decode(block.index_block_hash)?,
                    hex::decode(block.parent_block_id)?,
                    hex::decode(block.consensus_hash)?,
                    hex::decode(&parent.consensus_hash)?,
                ));
            } else {
                headers.push(BlockHeader::new(
                    block.block_height as u32,
                    hex::decode(block.index_block_hash)?,
                    hex::decode(block.parent_block_id)?,
                    hex::decode(block.consensus_hash)?,
                    vec![0_u8; 20],
                ));
            }

            current_block = block_parent;
        }

        headers.reverse();
        debug!("first block: {:?}", headers[0]);
        debug!("tip: {:?}", headers[headers.len() - 1]);
        debug!("retrieved {} block headers", headers.len());

        Ok(headers)
    }
}

impl<'a> RuntimeEnv<'a> for InstrumentedEnv<'a> {
    fn name(&self) -> &'a str {
        self.name
    }

    fn is_readonly(&self) -> bool {
        self.env_config.readonly
    }

    fn is_open(&self) -> bool {
        self.env_state.is_some()
    }

    fn open(&mut self) -> Result<()> {
        let name = self.name;
        let paths = &self.env_config.paths;
        let network = &self.env_config.network;

        info!("[{name}] opening environment...");
        paths.print(name);

        // Setup our options for the Marf.
        let mut marf_opts = stacks::MARFOpenOpts::default();
        marf_opts.external_blobs = true;

        // Setup our boot data to be used if the chainstate hasn't been initialized yet.
        let mut boot_data = if network.is_mainnet() {
            mainnet_boot_data()
        } else {
            todo!("testnet not yet supported")
        };

        debug!("initializing chainstate");
        let (chainstate, _) = stacks::StacksChainState::open_and_exec(
            network.is_mainnet(),
            1,
            &paths.chainstate_path,
            Some(&mut boot_data),
            Some(marf_opts.clone()),
        )?;
        info!("[{name}] chainstate initialized.");

        debug!("[{name}] loading index db...");
        let index_db_conn = SqliteConnection::establish(&paths.index_db_path)?;
        info!("[{name}] successfully connected to index db");

        debug!("[{name}] loading clarity db...");
        let clarity_db_conn = SqliteConnection::establish(&paths.clarity_db_path)?;
        info!("[{name}] successfully connected to clarity db");

        //debug!("attempting to migrate sortition db");
        debug!("opening sortition db");
        let sortition_db = super::open_sortition_db(&paths.sortition_db_path, network)?;
        info!("successfully opened sortition db");

        let state = InstrumentedEnvState {
            chainstate,
            index_db_conn: RefCell::new(index_db_conn),
            clarity_db_conn,
            sortition_db,
        };

        self.env_state = Some(state);

        ok!()
    }

    
}

/// Implementation of [ReadableEnv] for [InstrumentedEnv].
impl<'a> ReadableEnv<'a> for InstrumentedEnv<'a> {
    fn blocks(&self) -> Result<BlockCursor> {
        let headers = self.block_headers()?;
        let cursor = BlockCursor::new(&self.env_config.paths.blocks_dir, headers);
        Ok(cursor)
    }
}

/// Implementation of [WriteableEnv] for [InstrumentedEnv].
impl<'a> WriteableEnv<'a> for InstrumentedEnv<'a> {
    fn block_begin(
        &mut self,
        block: &crate::context::Block,
        f: impl FnOnce(&mut super::BlockTransactionContext) -> Result<()>,
    ) -> Result<()> {
        if self.is_readonly() {
            bail!("[{}] environment is read-only.", self.name);
        }

        let current_block_id: stacks::StacksBlockId;
        let next_block_id: stacks::StacksBlockId;

        match block {
            Block::Boot(_) => bail!("cannot process the boot block"),
            Block::Genesis(header) => {
                current_block_id = <stacks::StacksBlockId as stacks::ClarityMarfTrieId>::sentinel();
                next_block_id = header.stacks_block_id()?;
            }
            Block::Regular(header, _) => {
                current_block_id = header.parent_stacks_block_id()?;
                next_block_id = header.stacks_block_id()?;
            }
        };

        info!("current_block_id: {current_block_id}, next_block_id: {next_block_id}");

        // Insert this block into the app database.
        self.app_db.insert_block(
            self.id,
            block.block_height()? as i32,
            block.block_hash()?.as_bytes(),
            block.index_block_hash()?,
        )?;

        // We cannot process genesis as it was already processed as a part of chainstate
        // initialization. Log that we reached it and skip processing.
        if let Block::Genesis(_) = block {
            info!("genesis block cannot be processed as it was statically initialized; moving on");
            return ok!();
        }

        let state = self.get_env_state_mut()?;

        // Get an instance to the BurnStateDB (SortitionDB's `index_conn` implements this trait).
        let burn_db = state.sortition_db.index_conn();

        // Start a new chainstate transaction.
        debug!("creating chainstate tx");
        let (chainstate_tx, clarity_instance) = state.chainstate.chainstate_tx_begin()?;
        debug!("chainstate tx started");

        // Begin a new Clarity block.
        debug!("beginning clarity block");
        let mut clarity_block_conn = clarity_instance.begin_block(
            &current_block_id,
            &next_block_id,
            &chainstate_tx,
            &burn_db,
        );

        // Enter Clarity transaction processing for the new block.
        debug!("starting clarity tx processing");
        let clarity_tx_conn = clarity_block_conn.start_transaction_processing();

        // Call the provided function with our context object.
        //let mut block_tx_ctx = ;
        f(&mut BlockTransactionContext {
            clarity_tx_conn: &clarity_tx_conn,
        })?;

        debug!("returning");

        clarity_tx_conn.commit();
        clarity_block_conn.commit_to_block(&block.stacks_block_id()?);
        chainstate_tx.commit()?;

        ok!()
    }
}
