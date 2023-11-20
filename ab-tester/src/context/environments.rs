use std::fmt::Display;
use std::rc::Rc;

use color_eyre::eyre::anyhow;
use color_eyre::Result;

use self::instrumented::InstrumentedEnv;
use self::network::NetworkEnv;
use self::stacks_node::StacksNodeEnv;
use super::blocks::BlockCursor;
use super::{Block, BlockTransactionContext, Network, Runtime};
use crate::context::boot_data::mainnet_boot_data;
use crate::db::appdb::AppDb;
use crate::db::model;
use crate::{clarity, stacks};
use crate::types::*;

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
        InstrumentedEnv::new(
            env.id,
            name,
            Rc::clone(&self.app_db),
            working_dir,
            runtime,
            network,
        )
    }

    /// Creates and returns a new [NetworkEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn network(&self) -> Result<NetworkEnv> {
        todo!("the 'network' environment type is not currently implemented")
    }
}

pub struct RuntimeEnvContext {
    inner: Box<dyn ReadableEnv>,
}

impl RuntimeEnvContext {
    pub fn new<T: ReadableEnv + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
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
    inner: Box<dyn WriteableEnv>,
}

impl RuntimeEnvContextMut {
    pub fn new<T: WriteableEnv + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    pub fn import_burnstate(&mut self, source: &dyn ReadableEnv) -> Result<()> {
        todo!()
    }

    pub fn block_begin(&mut self, block: &Block) -> Result<BlockTransactionContext> {
        self.inner.block_begin(block)
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

    fn snapshots(&self) -> Result<Vec<Snapshot>> {
        self.inner.snapshots()
    }

    fn block_commits(&self) -> Result<Box<dyn Iterator<Item = Result<BlockCommit>>>> {
        todo!()
    }
}

impl ReadableEnv for RuntimeEnvContextMut {
    fn blocks(&self) -> Result<BlockCursor> {
        self.inner.blocks()
    }

    fn snapshots(&self) -> Result<Vec<Snapshot>> {
        self.inner.snapshots()
    }

    fn block_commits(&self) -> Result<Box<dyn Iterator<Item = Result<BlockCommit>>>> {
        todo!()
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

impl Display for &dyn RuntimeEnv {
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
    
    /// Retrieves all [Snapshot]s from the burnchain database.
    fn snapshots(&self) -> Result<Vec<Snapshot>>;

    /// Retrieves all [BlockCommit]s from the burnchain databases.
    fn block_commits<'a>(&'a self) -> Result<Box<dyn Iterator<Item = Result<BlockCommit>> + 'a>>;
}

/// Defines the functionality for a writeable [RuntimeEnv].
pub trait WriteableEnv: ReadableEnv {
    fn block_begin(&mut self, block: &Block) -> Result<BlockTransactionContext>;

    fn block_commit(
        &mut self,
        block_tx_ctx: BlockTransactionContext,
    ) -> Result<clarity::LimitedCostTracker>;

    fn import_snapshots(&mut self, snapshots: &[Snapshot]) -> Result<()>;
    fn import_block_commits(&mut self, block_commits: &[BlockCommit]) -> Result<()>;
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
