use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use super::{ReadableEnv, RuntimeEnv, WriteableEnv};
use crate::context::blocks::BlockHeader;
use crate::context::boot_data::mainnet_boot_data;
use crate::context::callbacks::{DefaultEnvCallbacks, RuntimeEnvCallbackHandler};
use crate::context::{
    Block, BlockCursor, BlockTransactionContext, Network, RegularBlockTransactionContext, Runtime,
    StacksEnvPaths,
};
use crate::db::appdb::AppDb;
use crate::db::model::app_db as model;
use crate::db::schema::appdb;
use crate::db::stacks_node::StacksNodeDb;
use crate::{clarity, ok, stacks};

/// Holds the configuration of an [InstrumentedEnv].
pub struct InstrumentedEnvConfig {
    working_dir: String,
    readonly: bool,
    paths: StacksEnvPaths,
    runtime: Runtime,
    network: Network,
}

/// Holds the opened state of an [InstrumentedEnv].
pub struct InstrumentedEnvState {
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    sortition_db: stacks::SortitionDB,
    burnstate_db: Box<dyn clarity::BurnStateDB>,
}

/// This environment type is app-specific and will instrument all Clarity-related
/// operations. This environment can be used for comparisons.
pub struct InstrumentedEnv {
    id: i32,
    name: String,
    app_db: Rc<AppDb>,
    env_config: InstrumentedEnvConfig,
    env_state: Option<InstrumentedEnvState>,
    callbacks: Box<dyn RuntimeEnvCallbackHandler>,
}

impl InstrumentedEnv {
    /// Creates a new [InstrumentedEnv]. This method expects the provided
    /// `working_dir` to either be uninitialized or be using the same [Runtime]
    /// and [Network] configuration.
    pub fn new(
        id: i32,
        name: String,
        app_db: Rc<AppDb>,
        working_dir: String,
        runtime: Runtime,
        network: Network,
    ) -> Result<Self> {
        let paths = StacksEnvPaths::new(&working_dir);

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
            callbacks: Box::<DefaultEnvCallbacks>::default(),
        })
    }

    fn readonly(&mut self, readonly: bool) {
        self.env_config.readonly = readonly;
    }

    /// Attempts to retrieve the [InstrumentedEnvState] for this environment. Will
    /// return an error if [RuntimeEnv::open] has not been called.
    fn get_env_state(&self) -> Result<&InstrumentedEnvState> {
        let state = self
            .env_state
            .as_ref()
            .ok_or(anyhow!("[{}] environment has not been opened", self.name))?;

        Ok(state)
    }

    /// Attempts to retrieve the [InstrumentedEnvState] for this environment as a
    /// mutable reference. Will return an error if [RuntimeEnv::open] has not been called.
    fn get_env_state_mut(&mut self) -> Result<&mut InstrumentedEnvState> {
        let state = self
            .env_state
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
        self.callbacks.get_chain_tip_start(self);
        let tip = appdb::_block_headers::table
            .order_by(appdb::_block_headers::block_height.desc())
            .limit(1)
            .get_result::<model::BlockHeader>(&mut *state.index_db_conn.borrow_mut())?;
        // TODO: Handle when there is no tip (chain uninitialized).
        self.callbacks
            .get_chain_tip_finish(self, tip.block_height as u32);
        let mut current_block = Some(tip);

        // Vec for holding the headers we run into. This will initially be
        // in reverse order (from tip to genesis) - we reverse it later.
        let mut headers: Vec<BlockHeader> = Vec::new();

        // Walk backwards from tip to genesis, following the canonical fork. We
        // do this so that we don't follow orphaned blocks/forks.
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
            self.callbacks.load_block_headers_iter(self, headers.len());

            current_block = block_parent;
        }

        // Reverse the vec so that it is in block-ascending order.
        headers.reverse();

        debug!("first block: {:?}", headers[0]);
        debug!("tip: {:?}", headers[headers.len() - 1]);
        debug!("retrieved {} block headers", headers.len());

        self.callbacks
            .load_block_headers_finish(self, headers.len());
        Ok(headers)
    }
}

/// Implementation of [RuntimeEnv] for [InstrumentedEnv].
impl RuntimeEnv for InstrumentedEnv {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn is_readonly(&self) -> bool {
        self.env_config.readonly
    }

    fn is_open(&self) -> bool {
        self.env_state.is_some()
    }

    fn open(&mut self) -> Result<()> {
        let name = &self.name;
        let paths = &self.env_config.paths;
        let network = &self.env_config.network;

        info!("[{name}] opening environment...");
        self.callbacks.env_open_start(self, name);
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
        self.callbacks
            .open_chainstate_start(self, &paths.chainstate_dir);
        let (chainstate, _) = stacks::StacksChainState::open_and_exec(
            network.is_mainnet(),
            1,
            &paths.chainstate_dir,
            Some(&mut boot_data),
            Some(marf_opts.clone()),
        )?;
        self.callbacks.open_chainstate_finish(self);
        info!("[{name}] chainstate initialized.");

        debug!("[{name}] loading index db...");
        self.callbacks
            .open_index_db_start(self, &paths.index_db_path);
        let index_db_conn = SqliteConnection::establish(&paths.index_db_path)?;
        self.callbacks.open_index_db_finish(self);
        info!("[{name}] successfully connected to index db");

        debug!("[{name}] loading clarity db...");
        self.callbacks
            .open_clarity_db_start(self, &paths.clarity_db_path);
        let clarity_db_conn = SqliteConnection::establish(&paths.clarity_db_path)?;
        self.callbacks.open_clarity_db_finish(self);
        info!("[{name}] successfully connected to clarity db");

        //debug!("attempting to migrate sortition db");
        debug!("opening sortition db");
        self.callbacks
            .open_sortition_db_start(self, &paths.sortition_dir);
        let sortition_db = super::open_sortition_db(&paths.sortition_dir, network)?;
        self.callbacks.open_sortition_db_finish(self);
        info!("successfully opened sortition db");

        let burnstate_db: Box<dyn clarity::BurnStateDB> = Box::new(StacksNodeDb::new(
            &paths.index_db_path,
            &paths.sortition_db_path,
        )?);

        let state = InstrumentedEnvState {
            chainstate,
            index_db_conn: RefCell::new(index_db_conn),
            clarity_db_conn,
            sortition_db,
            burnstate_db,
        };

        self.env_state = Some(state);

        ok!()
    }
}

/// Implementation of [ReadableEnv] for [InstrumentedEnv].
impl ReadableEnv for InstrumentedEnv {
    fn blocks(&self) -> Result<BlockCursor> {
        let headers = self.block_headers()?;
        let cursor = BlockCursor::new(&self.env_config.paths.blocks_dir, headers);
        Ok(cursor)
    }
}

/// Implementation of [WriteableEnv] for [InstrumentedEnv].
impl WriteableEnv for InstrumentedEnv {
    fn block_begin(&mut self, block: &crate::context::Block) -> Result<BlockTransactionContext> {
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
            return Ok(BlockTransactionContext::Genesis);
        }

        let state = self.get_env_state_mut()?;

        // Get an instance to the BurnStateDB (SortitionDB's `index_conn` implements this trait).

        let mut chainstate_tx = state.chainstate.state_index.begin_tx()?;
        chainstate_tx.begin(&current_block_id, &next_block_id)?;


        todo!();

        let clarity_block_conn = state.chainstate.clarity_state.begin_block(
            &current_block_id,
            &next_block_id,
            &state.chainstate.state_index,
            &*state.burnstate_db,
        );

        // Start a new chainstate transaction.
        //debug!("creating chainstate tx");
        //let (chainstate_tx, clarity_instance) = state.chainstate.chainstate_tx_begin()?;
        //debug!("chainstate tx started");

        // Begin a new Clarity block.
        //debug!("beginning clarity block");
        /*let mut clarity_block_conn = clarity_instance.begin_block(
            &current_block_id,
            &next_block_id,
            &state.chainstate.state_index,
            &*state.burnstate_db,
        );*/

        // Enter Clarity transaction processing for the new block.
        debug!("starting clarity tx processing");
        //let clarity_tx_conn = ;

        // Call the provided function with our context object.
        //let mut block_tx_ctx = ;
        let block_tx_ctx =
            RegularBlockTransactionContext::new(block.stacks_block_id()?, clarity_block_conn);

        Ok(BlockTransactionContext::Regular(block_tx_ctx))
    }

    fn block_commit(
        &mut self,
        block_tx_ctx: BlockTransactionContext,
    ) -> Result<clarity::LimitedCostTracker> {
        if let BlockTransactionContext::Regular(ctx) = block_tx_ctx {
            //clarity_tx_conn.commit();
            //let cost_tracker = clarity_block_conn.commit_to_block(&stacks_block_id);
            //chainstate_tx.commit()?;
        } else {
            bail!("Cannot commit genesis block as it has already been statically loaded from boot data.")
        }

        todo!()
    }
}
