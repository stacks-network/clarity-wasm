use std::cell::RefCell;

use color_eyre::{eyre::{anyhow, bail}, Result};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use crate::{
    context::{
        blocks::BlockHeader, boot_data::mainnet_boot_data, BlockCursor, Network, Runtime,
        TestEnvPaths
    },
    db::model::app_db as model,
    db::schema::appdb,
    ok, stacks,
};

use super::{ReadableEnv, RuntimeEnv, WriteableEnv};

pub struct InstrumentedEnvConfig<'a> {
    working_dir: &'a str,
    readonly: bool,
    paths: TestEnvPaths,
    runtime: Runtime,
    network: Network,
}

pub struct InstrumentedEnvState {
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    sortition_db: stacks::SortitionDB,
}

/// This environment type is app-specific and will instrument all Clarity-related
/// operations. This environment can be used for comparisons.
pub struct InstrumentedEnv<'a> {
    name: &'a str,
    env_config: InstrumentedEnvConfig<'a>,
    env_state: Option<InstrumentedEnvState>,
}

impl<'a> InstrumentedEnv<'a> {
    /// Creates a new [InstrumentedEnv]. This method expects the provided
    /// `working_dir` to either be uninitialized or be using the same [Runtime]
    /// and [Network] configuration.
    pub fn new(
        name: &'a str,
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
            name,
            env_config,
            env_state: None,
        })
    }

    fn readonly(&mut self, readonly: bool) {
        self.env_config.readonly = readonly;
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

impl<'a> ReadableEnv<'a> for InstrumentedEnv<'a> {
    fn blocks(&self) -> Result<BlockCursor> {
        let headers = self.block_headers()?;
        let cursor = BlockCursor::new(&self.env_config.paths.blocks_dir, headers);
        Ok(cursor)
    }
}

impl<'a> WriteableEnv<'a> for InstrumentedEnv<'a> {
    fn block_begin(
        &mut self,
        block: &crate::context::Block,
        f: impl FnOnce(&mut super::BlockTransactionContext) -> Result<()>,
    ) -> Result<()> {
        if self.is_readonly() {
            bail!("[{}] environment is read-only.", self.name);
        }

        todo!()
    }
}
