use std::{
    cell::RefCell,
    fmt::Display,
};

use color_eyre::{
    eyre::{bail, anyhow},
    Result,
};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use crate::{
    clarity,
    clarity::ClarityConnection,
    context::boot_data::mainnet_boot_data,
    db::{appdb::AppDb, datastore::DataStore, model, schema},
    ok, stacks,
};

use self::{instrumented::InstrumentedEnv, network::NetworkEnv, stacks_node::StacksNodeEnv};

use super::{
    blocks::{BlockCursor, BlockHeader},
    Block, Network, Runtime, StoreType, TestEnvPaths,
};

mod instrumented;
mod network;
mod stacks_node;

/// Helper struct for creating new [RuntimeEnv] instances.
pub struct RuntimeEnvBuilder<'a> {
    app_db: &'a AppDb,
}

impl<'a> RuntimeEnvBuilder<'a> {
    pub fn new(app_db: &'a AppDb) -> Self {
        Self { app_db }
    }

    fn get_or_create_env(&self, name: &'a str) -> Result<model::app_db::Environment> {
        self.app_db.get_env(name)?
            .or_else(|| self.app_db.insert_environment(Runtime::None as i32, name).ok())
            .ok_or(anyhow!("failed to get or create runtime environment"))
    }

    /// Creates and returns a new [StacksNodeEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn stacks_node(&self, name: &'a str, node_dir: &'a str) -> Result<StacksNodeEnv<'a>> {
        let env = self.get_or_create_env(name)?;
        StacksNodeEnv::new(env.id, name, node_dir)
    }

    /// Creates and returns a new [InstrumentedEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn instrumented(&self,
        name: &'a str,
        runtime: Runtime,
        network: Network,
        working_dir: &'a str,
    ) -> Result<InstrumentedEnv<'a>> {
        let env = self.get_or_create_env(name)?;
        InstrumentedEnv::new(env.id, name, self.app_db, working_dir, runtime, network)
    }

    /// Creates and returns a new [NetworkEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn network(&self) -> Result<NetworkEnv<'a>> {
        todo!("the 'network' environment type is not currently implemented")
    }
}

/// Defines the basic functionality for a [RuntimeEnv] implementation.
pub trait RuntimeEnv<'a> {
    /// Gets the user-provided name of this environment.
    fn name(&self) -> &'a str;
    /// Gets whether or not this environment is read-only. Note that some environment
    /// types are inherently read-only, meanwhile others can be configured
    /// independently.
    fn is_readonly(&self) -> bool;
    /// Gets whether or not this environment has been opened/initialized.
    fn is_open(&self) -> bool;
    /// Opens the environment and initializes it if needed (and writeable).
    fn open(&mut self) -> Result<()>;
}

/// Defines the functionality for a readable [RuntimeEnv].
pub trait ReadableEnv<'a>: RuntimeEnv<'a> {
    /// Provides a [BlockCursor] over the Stacks blocks contained within this 
    /// environment.
    fn blocks(&self) -> Result<BlockCursor>;
}

/// Defines the functionality for a writeable [RuntimeEnv].
pub trait WriteableEnv<'a>: ReadableEnv<'a> {
    fn block_begin(
        &mut self,
        block: &Block,
        f: impl FnOnce(&mut BlockTransactionContext) -> Result<()>,
    ) -> Result<()>
    where
        Self: Sized;
}

impl Display for &dyn RuntimeEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Display for &mut dyn RuntimeEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub struct BlockTransactionContext<'a, 'b> {
    clarity_tx_conn: &'a stacks::ClarityTransactionConnection<'a, 'b>,
}

/// Opens the sortition DB baseed on the provided network.
fn open_sortition_db(path: &str, network: &Network) -> Result<stacks::SortitionDB> {
    match network {
        Network::Mainnet(_) => {
            let boot_data = mainnet_boot_data();

            let sortition_db = stacks::SortitionDB::connect(
                path,
                stacks::BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
                &stacks::BurnchainHeaderHash::from_hex(stacks::BITCOIN_MAINNET_FIRST_BLOCK_HASH)
                    .unwrap(),
                stacks::BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP.into(),
                stacks::STACKS_EPOCHS_MAINNET.as_ref(),
                boot_data.pox_constants,
                true,
            )?;

            Ok(sortition_db)
        }
        Network::Testnet(_) => {
            todo!("testnet not yet supported")
        }
    }
}
