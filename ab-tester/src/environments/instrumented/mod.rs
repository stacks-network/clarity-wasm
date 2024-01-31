use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use color_eyre::eyre::anyhow;
use color_eyre::Result;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use super::stacks_node::StacksEnvPaths;
use super::{EnvConfig, EnvPaths, RuntimeEnv};
use crate::context::boot_data::mainnet_boot_data;
use crate::context::callbacks::{DefaultEnvCallbacks, RuntimeEnvCallbackHandler};
use crate::context::{Network, Runtime};
use crate::db::appdb::burnstate_db::{AppDbBurnStateWrapper, AsBurnStateDb};
use crate::db::appdb::headers_db::AsHeadersDb;
use crate::db::appdb::AppDb;
use crate::db::{model, stacks_instrumentation};
use crate::db::schema::{self};
use crate::environments::stacks_node::db::stacks_headers_db::StacksHeadersDb;
use crate::types::BlockHeader;
use crate::{clarity, ok, stacks};

pub mod readable_env;
pub mod writeable_env;

/// Holds the configuration of an [InstrumentedEnv].
pub struct InstrumentedEnvConfig {
    working_dir: PathBuf,
    readonly: bool,
    paths: Box<dyn EnvPaths>,
    runtime: Runtime,
    network: Network,
}

impl EnvConfig for InstrumentedEnvConfig {
    fn working_dir(&self) -> &std::path::Path {
        &self.working_dir
    }
    fn chainstate_index_db_path(&self) -> &std::path::Path {
        self.paths.index_db_path()
    }

    fn is_chainstate_app_indexed(&self) -> bool {
        false
    }

    fn sortition_dir(&self) -> &std::path::Path {
        self.paths.sortition_dir()
    }

    fn sortition_db_path(&self) -> &std::path::Path {
        self.paths.sortition_db_path()
    }

    fn is_sortition_app_indexed(&self) -> bool {
        true
    }

    fn clarity_db_path(&self) -> &std::path::Path {
        self.paths.clarity_db_path()
    }

    fn is_clarity_db_app_indexed(&self) -> bool {
        true
    }

    fn blocks_dir(&self) -> &std::path::Path {
        self.paths.blocks_dir()
    }
}

/// Holds the opened state of an [InstrumentedEnv].
pub struct InstrumentedEnvState {
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    //sortition_db_conn: Rc<RefCell<SqliteConnection>>,
    //sortition_db: stacks::SortitionDB,
    burnstate_db: Box<dyn clarity::BurnStateDB>,
    headers_db: Box<dyn clarity::HeadersDB>,
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
        working_dir: PathBuf,
        runtime: Runtime,
        network: Network,
    ) -> Self {
        let paths = StacksEnvPaths::new(working_dir.clone());

        let env_config = InstrumentedEnvConfig {
            working_dir,
            readonly: false,
            paths: Box::new(paths),
            runtime,
            network,
        };

        Self {
            id,
            name,
            app_db,
            env_config,
            env_state: None,
            callbacks: Box::<DefaultEnvCallbacks>::default(),
        }
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
    fn block_headers(&self, max_blocks: Option<u32>) -> Result<Vec<BlockHeader>> {
        // Get our state
        let state = self
            .env_state
            .as_ref()
            .ok_or(anyhow!("environment has not been opened"))?;

        // Retrieve the tip.
        self.callbacks.get_chain_tip_start(self);
        let tip = schema::_block_headers::table
            .order_by(schema::_block_headers::block_height.desc())
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
            let block_parent = schema::_block_headers::table
                .filter(schema::_block_headers::index_block_hash.eq(&block.parent_block_id))
                .get_result::<model::BlockHeader>(&mut *state.index_db_conn.borrow_mut())
                .optional()?;

            headers.push(block.try_into()?);
            self.callbacks.load_block_headers_iter(self, headers.len());

            current_block = block_parent;
        }

        // Reverse the vec so that it is in block-ascending order.
        headers.reverse();

        // If we have a max-block limit, avoid returning more than necessary.
        if let Some(max_blocks) = max_blocks {
            headers = headers
                .into_iter()
                .take(max_blocks as usize)
                .collect();
        }

        //debug!("first block: {:?}", headers[0]);
        //debug!("tip: {:?}", headers[headers.len() - 1]);
        debug!("retrieved {} block headers", headers.len());

        self.callbacks
            .load_block_headers_finish(self, headers.len());
        Ok(headers)
    }
}

impl AsHeadersDb for InstrumentedEnv {
    fn as_headers_db(&self) -> Result<&dyn clarity::HeadersDB> {
        let state = self.get_env_state()?;
        Ok(&*state.headers_db)
    }
}

impl AsBurnStateDb for InstrumentedEnv {
    fn as_burnstate_db(&self) -> Result<&dyn clarity::BurnStateDB> {
        let state = self.get_env_state()?;
        Ok(&*state.burnstate_db)
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
        self.callbacks
            .env_open_start(self, &self.env_config.working_dir);
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
            .open_chainstate_start(self, paths.chainstate_dir());
        let (chainstate, _) = stacks::StacksChainState::open_and_exec(
            network.is_mainnet(),
            1,
            &paths.chainstate_dir().display().to_string(),
            Some(&mut boot_data),
            Some(marf_opts.clone()),
        )?;
        self.callbacks.open_chainstate_finish(self);
        info!("[{name}] chainstate initialized.");

        debug!("[{name}] loading index db...");
        self.callbacks
            .open_index_db_start(self, paths.index_db_path());
        let index_db_conn =
            SqliteConnection::establish(&paths.index_db_path().display().to_string())?;
        self.callbacks.open_index_db_finish(self);
        info!("[{name}] successfully connected to index db");

        // Open the Clarity sqlite db
        debug!("[{name}] loading clarity db...");
        self.callbacks
            .open_clarity_db_start(self, paths.clarity_db_path());
        let mut clarity_db_conn =
            SqliteConnection::establish(&paths.clarity_db_path().display().to_string())?;
        debug!("[{name}] installing instrumentation tables and triggers to the clarity db...");
        stacks_instrumentation::install_clarity_db_instrumentation(&mut clarity_db_conn)?;
        self.callbacks.open_clarity_db_finish(self);
        info!("[{name}] successfully connected to clarity db");

        let burnstate_db =
            AppDbBurnStateWrapper::new(self.id, self.app_db.clone(), boot_data.pox_constants);

        //let headers_db = AppDbHeadersWrapper::new(self.id, self.app_db.clone());

        // Open the burnstate db
        /*
        let burnstate_db = StacksBurnStateDb::new(
            &paths.sortition_db_path,
            boot_data.pox_constants,
        )?;
        */

        // Open the headers db
        let headers_db = StacksHeadersDb::new(paths.index_db_path())?;

        let state = InstrumentedEnvState {
            chainstate,
            index_db_conn: RefCell::new(index_db_conn),
            clarity_db_conn,
            burnstate_db: Box::new(burnstate_db),
            headers_db: Box::new(headers_db),
        };

        self.env_state = Some(state);

        ok!()
    }

    fn id(&self) -> i32 {
        self.id
    }

    fn cfg(&self) -> &dyn EnvConfig {
        &self.env_config
    }
}