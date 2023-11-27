mod instrumented;
mod network;
mod stacks_node;

use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use color_eyre::eyre::anyhow;
use color_eyre::Result;

use crate::context::{Runtime, Network, Block, BlockTransactionContext, BlockCursor};
use crate::context::boot_data::mainnet_boot_data;
use crate::db::appdb::AppDb;
use crate::db::model;
use crate::types::*;
use crate::{clarity, stacks};

use self::instrumented::InstrumentedEnv;
use self::network::NetworkEnv;
use self::stacks_node::StacksNodeEnv;

pub type BoxedDbIterResult<Model> = Result<Box<dyn Iterator<Item = Result<Model>>>>;

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

/// Represents a readable runtime environment, such as an existing Stacks node
/// data dir.
pub struct RuntimeEnvContext {
    inner: Box<dyn ReadableEnv>,
}

impl Deref for RuntimeEnvContext {
    type Target = dyn ReadableEnv;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl DerefMut for RuntimeEnvContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl RuntimeEnvContext {
    pub fn new<T: ReadableEnv + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    pub fn id(&self) -> i32 {
        self.inner.id()
    }

    pub fn as_readable_env(&self) -> &dyn ReadableEnv {
        &*self.inner
    }
}

/// Represents a mutable/writable environment. Required for target environments
/// to be opened using this type, which wraps [WriteableEnv].
pub struct RuntimeEnvContextMut {
    inner: Box<dyn WriteableEnv>,
}

impl Deref for RuntimeEnvContextMut {
    type Target = dyn WriteableEnv;

    fn deref(&self) -> &Self::Target {
        &*self.inner as &dyn WriteableEnv
    }
}

impl DerefMut for RuntimeEnvContextMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner as &mut dyn WriteableEnv
    }
}

impl RuntimeEnvContextMut {
    pub fn new<T: WriteableEnv + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    pub fn block_begin(&mut self, block: &Block) -> Result<BlockTransactionContext> {
        self.inner.block_begin(block)
    }
}

/// Defines the basic functionality for a [RuntimeEnv] implementation.
pub trait RuntimeEnv {
    /// Gets the system-assigned id for this environment.
    fn id(&self) -> i32;
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

    /// Retrieves all [Snapshot]s from the burnchain sortition datastore.
    fn snapshots(&self) -> BoxedDbIterResult<Snapshot>;

    /// Retrieves all [BlockCommit]s from the burnchain sortition datastore.
    fn block_commits(&self) -> BoxedDbIterResult<BlockCommit>;

    /// Retrieves all [AstRuleHeight]s from the burnchain sortition datastore.
    fn ast_rules(&self) -> BoxedDbIterResult<AstRuleHeight>;

    /// Retrieves all [Epoch]s from the burnchain sortition datastore.
    fn epochs(&self) -> BoxedDbIterResult<Epoch>;
}

/// Defines the functionality for a writeable [RuntimeEnv].
pub trait WriteableEnv: ReadableEnv {
    /// Begins the [Block] from the source environment in the target environment's
    /// chainstate.
    fn block_begin(&mut self, block: &Block) -> Result<BlockTransactionContext>;

    /// Commits the currently open [Block] from the source environment to the
    /// target environment's chainstate.
    fn block_commit(
        &mut self,
        block_tx_ctx: BlockTransactionContext,
    ) -> Result<clarity::LimitedCostTracker>;

    fn import_burnstate(&self, source: &dyn ReadableEnv) -> Result<()>;
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
