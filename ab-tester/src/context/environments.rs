use std::{fmt::Display, ops::Deref, rc::Rc};

use color_eyre::{eyre::anyhow, Result};

use crate::{
    context::boot_data::mainnet_boot_data,
    db::{appdb::AppDb, model},
    stacks,
};

use self::{instrumented::InstrumentedEnv, network::NetworkEnv, stacks_node::StacksNodeEnv};

use super::{blocks::BlockCursor, Block, Network, Runtime};

pub mod instrumented;
pub mod network;
pub mod stacks_node;

/// Helper struct for creating new [RuntimeEnv] instances.
pub struct RuntimeEnvBuilder {
    app_db: Rc<AppDb>,
}

impl RuntimeEnvBuilder {
    pub fn new(app_db: Rc<AppDb>) -> Self {
        Self { app_db }
    }

    fn get_or_create_env(
        &self,
        name: &str,
        runtime: &Runtime,
        path: &str,
    ) -> Result<model::app_db::Environment> {
        self.app_db
            .get_env(name)?
            .or_else(|| {
                self.app_db
                    .insert_environment(runtime.into(), name, path)
                    .ok()
            })
            .ok_or(anyhow!("failed to get or create runtime environment"))
    }

    /// Creates and returns a new [StacksNodeEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn stacks_node(&self, name: String, node_dir: String) -> Result<StacksNodeEnv> {
        let env = self.get_or_create_env(&name, &Runtime::None, &node_dir)?;
        StacksNodeEnv::new(env.id, name, node_dir)
    }

    /// Creates and returns a new [InstrumentedEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn instrumented(
        &self,
        name: String,
        runtime: Runtime,
        network: Network,
        working_dir: String,
    ) -> Result<InstrumentedEnv> {
        let env = self.get_or_create_env(&name, &runtime, &working_dir)?;
        InstrumentedEnv::new(env.id, name, Rc::clone(&self.app_db), working_dir, runtime, network)
    }

    /// Creates and returns a new [NetworkEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn network(&self) -> Result<NetworkEnv> {
        todo!("the 'network' environment type is not currently implemented")
    }
}

pub struct RuntimeEnvContext {
    inner: Box<dyn ReadableEnv>
}

impl RuntimeEnvContext {
    pub fn new<T: ReadableEnv + 'static>(inner: T) -> Self {
        Self { 
            inner: Box::new(inner)
        }
    }
}

impl RuntimeEnv for RuntimeEnvContext {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn is_readonly(&self) -> bool {
        self.inner.is_readonly()
    }

    fn is_open(&self) -> bool {
        self.inner.is_open()
    }

    fn open(&mut self) -> Result<()> {
        self.inner.open()
    }
}

pub struct RuntimeEnvContextMut {
    inner: Box<dyn WriteableEnv>
}

impl RuntimeEnvContextMut {
    pub fn new<T: WriteableEnv + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner)
        }
    }
}

impl RuntimeEnv for RuntimeEnvContextMut {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn is_readonly(&self) -> bool {
        self.inner.is_readonly()
    }

    fn is_open(&self) -> bool {
        self.inner.is_open()
    }

    fn open(&mut self) -> Result<()> {
        self.inner.open()
    }
}

impl ReadableEnv for RuntimeEnvContext {
    fn blocks(&self) -> Result<BlockCursor> {
        self.inner.blocks()
    }
}

/// Defines the basic functionality for a [RuntimeEnv] implementation.
pub trait RuntimeEnv {
    /// Gets the user-provided name of this environment.
    fn name(&self) -> String;
    /// Gets whether or not this environment is read-only. Note that some environment
    /// types are inherently read-only, meanwhile others can be configured
    /// independently.
    fn is_readonly(&self) -> bool;
    /// Gets whether or not this environment has been opened/initialized.
    fn is_open(&self) -> bool;
    /// Opens the environment and initializes it if needed (and writeable).
    fn open(&mut self) -> Result<()>;
}

impl<'a> Display for &dyn RuntimeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Display for &mut dyn RuntimeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Defines the functionality for a readable [RuntimeEnv].
pub trait ReadableEnv: RuntimeEnv {
    /// Provides a [BlockCursor] over the Stacks blocks contained within this
    /// environment.
    fn blocks(&self) -> Result<BlockCursor>;
}

/// Defines the functionality for a writeable [RuntimeEnv].
pub trait WriteableEnv: ReadableEnv {
    fn block_begin(
        &mut self,
        block: &Block,
        f: impl FnOnce(&mut BlockTransactionContext) -> Result<()>,
    ) -> Result<()>
    where
        Self: Sized;
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
