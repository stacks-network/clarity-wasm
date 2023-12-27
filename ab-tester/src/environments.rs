pub mod instrumented;
pub mod network;
pub mod stacks_node;

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use color_eyre::eyre::anyhow;
use color_eyre::Result;
use log::*;

use self::instrumented::InstrumentedEnv;
use self::network::NetworkEnv;
use self::stacks_node::StacksNodeEnv;
use crate::context::boot_data::mainnet_boot_data;
use crate::context::{Block, BlockCursor, Network, Runtime};
use crate::db::appdb::AppDb;
use crate::types::*;
use crate::{clarity, stacks};

pub type BoxedDbIterResult<Model> = Result<Box<dyn Iterator<Item = Result<Model>>>>;

/// Helper struct for creating new [RuntimeEnv] instances.
pub struct RuntimeEnvBuilder {
    app_db: Rc<AppDb>,
}

impl RuntimeEnvBuilder {
    pub fn new(app_db: Rc<AppDb>) -> Self {
        Self { app_db }
    }

    fn get_env_by_name(
        &self,
        name: &str
    ) -> Result<Option<super::db::model::Environment>> {
        let result = self.app_db
            .get_env_by_name(name)?;
        Ok(result)
    }

    fn get_or_create_env(
        &self,
        name: &str,
        runtime: &Runtime,
        network: &Network,
        readonly: bool,
        path: &str,
    ) -> Result<super::db::model::Environment> {
        self.app_db
            .get_env_by_name(name)?
            .or_else(|| {
                self.app_db
                    .insert_environment(
                        runtime.into(), 
                        network.network_id() as i32,
                        network.chain_id() as i32,
                        readonly, 
                        name, 
                        path)
                    .ok()
            })
            .ok_or(anyhow!("failed to get or create runtime environment"))
    }

    /// Creates and returns a new [StacksNodeEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn stacks_node(&self, name: String, node_dir: PathBuf) -> Result<StacksNodeEnv> {
        let network = StacksNodeEnv::read_network_from_db_config(&node_dir)?;

        let env = self.get_or_create_env(
            &name,
            &Runtime::None,
            &network,
            true,
            node_dir
                .to_str()
                .ok_or(anyhow!("failed to convert node dir to path"))?,
        )?;
        Ok(StacksNodeEnv::new(env.id, name, node_dir))
    }

    /// Creates and returns a new [InstrumentedEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn instrumented(
        &self,
        name: String,
        runtime: Runtime,
        network: Network,
        readonly: bool,
        working_dir: String,
    ) -> Result<InstrumentedEnv> {
        let env = self.get_or_create_env(
            &name, 
            &runtime, 
            &network, 
            readonly, 
            &working_dir
        )?;

        Ok(InstrumentedEnv::new(
            env.id,
            name,
            Rc::clone(&self.app_db),
            working_dir.into(),
            runtime,
            network,
        ))
    }

    /// Creates and returns a new [NetworkEnv] with the provided configuration.
    /// Note that [RuntimeEnv::open] must be called on the environment prior to
    /// using it.
    pub fn network(&self) -> Result<NetworkEnv> {
        todo!("the 'network' environment type is not currently implemented")
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
    /// Retrieves the paths used by this [RuntimeEnv].
    fn cfg(&self) -> &dyn EnvConfig;
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
    fn blocks(&self, max_blocks: Option<u32>) -> Result<BlockCursor>;

    fn last_block_height(&self) -> Result<u32>;

    // TODO: Move environment data export methods to their own trait.

    /// Retrieves all [Snapshot]s from the burnchain sortition datastore.
    fn snapshots(&self, prefetch_limit: u32) -> BoxedDbIterResult<Snapshot>;
    fn snapshot_count(&self) -> Result<usize>;

    /// Retrieves all [BlockCommit]s from the burnchain sortition datastore.
    fn block_commits(&self, prefetch_limit: u32) -> BoxedDbIterResult<BlockCommit>;
    fn block_commit_count(&self) -> Result<usize>;

    /// Retrieves all [AstRuleHeight]s from the burnchain sortition datastore.
    fn ast_rules(&self) -> BoxedDbIterResult<AstRuleHeight>;
    fn ast_rule_count(&self) -> Result<usize>;

    /// Retrieves all [Epoch]s from the burnchain sortition datastore.
    fn epochs(&self) -> BoxedDbIterResult<Epoch>;
    fn epoch_count(&self) -> Result<usize>;

    /// Retrieves all [BlockHeader]s from the chainstate index.
    fn block_headers(&self, prefetch_limit: u32) -> BoxedDbIterResult<BlockHeader>;
    fn block_header_count(&self) -> Result<usize>;

    fn payments(&self, prefetch_limit: u32) -> BoxedDbIterResult<Payment>;
    fn payment_count(&self) -> Result<usize>;
}

pub struct ClarityBlockTransactionResult {
    asset_map: clarity::AssetMap,
    tx_events: Vec<stacks::StacksTransactionEvent>,
    aborted: bool,
}

impl ClarityBlockTransactionResult {
    pub fn new(
        asset_map: clarity::AssetMap,
        tx_events: Vec<stacks::StacksTransactionEvent>,
        aborted: bool,
    ) -> Self {
        Self {
            asset_map,
            tx_events,
            aborted,
        }
    }
}

/// Defines the functionality for a writeable [RuntimeEnv].
pub trait WriteableEnv: ReadableEnv {
    /// Begins the [Block] from the source environment in the target environment's
    /// chainstate.
    //fn block_begin(&mut self, block: &Block) -> Result<&mut BlockContext>;

    fn process_block(&mut self, block: &Block) -> Result<()>;

    // TODO: Move environment data import methods to their own trait.
    fn import_burnstate(&self, source: &dyn ReadableEnv) -> Result<()>;
    fn import_chainstate(&self, source: &dyn ReadableEnv) -> Result<()>;

    fn clear_blocks(&self) -> Result<u32>;

    fn as_readable_env(&self) -> &dyn ReadableEnv
    where
        Self: Sized,
    {
        self as &dyn ReadableEnv
    }
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

pub trait EnvPaths {
    fn index_db_path(&self) -> &Path;
    fn sortition_dir(&self) -> &Path;
    fn sortition_db_path(&self) -> &Path;
    fn blocks_dir(&self) -> &Path;
    fn chainstate_dir(&self) -> &Path;
    fn clarity_db_path(&self) -> &Path;

    /// Prints information about the paths.
    fn print(&self, env_name: &str) {
        info!("[{env_name}] using directories:");
        debug!("[{env_name}] index db: {:?}", self.index_db_path());
        debug!("[{env_name}] sortition dir: {:?}", self.sortition_dir());
        debug!("[{env_name}] sortition db: {:?}", self.sortition_db_path());
        debug!("[{env_name}] clarity db: {:?}", self.clarity_db_path());
        debug!("[{env_name}] blocks dir: {:?}", self.blocks_dir());
        debug!("[{env_name}] chainstate dir: {:?}", self.chainstate_dir());
    }
}

pub trait EnvConfig {
    fn chainstate_index_db_path(&self) -> &Path;
    fn is_chainstate_app_indexed(&self) -> bool;
    fn blocks_dir(&self) -> &Path;

    fn sortition_dir(&self) -> &Path;
    fn sortition_db_path(&self) -> &Path;
    fn is_sortition_app_indexed(&self) -> bool;

    fn clarity_db_path(&self) -> &Path;
    fn is_clarity_db_app_indexed(&self) -> bool;
}
