use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::rc::Rc;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use log::*;

use crate::config::Config;
use crate::context::replay::ChainStateReplayer;
use crate::db::appdb::AppDb;
use crate::utils::append_to_path;
use crate::{clarity, ok, stacks};

pub mod blocks;
pub mod boot_data;
pub mod callbacks;
pub mod replay;

pub use blocks::{Block, BlockCursor};

use self::callbacks::ReplayCallbackHandler;
use self::replay::{ReplayOpts, ReplayResult};
use crate::environments::{ReadableEnv, RuntimeEnvBuilder, WriteableEnv};

pub struct BaselineBuilder<'ctx> {
    app_db: Rc<AppDb>,
    baseline_env: Option<Box<dyn ReadableEnv + 'ctx>>,
}

impl<'ctx> BaselineBuilder<'ctx> {
    fn new(app_db: Rc<AppDb>) -> Self {
        Self {
            app_db,
            baseline_env: None,
        }
    }

    pub fn stacks_node(mut self, name: &'_ str, node_dir: PathBuf) -> Result<Self> {
        let env =
            RuntimeEnvBuilder::new(self.app_db.clone()).stacks_node(name.to_string(), node_dir)?;
        self.baseline_env = Some(Box::new(env));
        Ok(self)
    }
}

pub struct InstrumentIntoBuilder<'ctx> {
    app_db: Rc<AppDb>,
    instrumented_envs: Vec<Box<dyn WriteableEnv + 'ctx>>,
}

impl<'ctx> InstrumentIntoBuilder<'ctx> {
    fn new(app_db: Rc<AppDb>) -> Self {
        Self {
            app_db,
            instrumented_envs: Vec::new(),
        }
    }

    pub fn instrumented(
        mut self,
        name: &'ctx str,
        runtime: Runtime,
        network: Network,
        working_dir: &'ctx str,
    ) -> Result<InstrumentIntoBuilder<'ctx>> {
        let env_builder = RuntimeEnvBuilder::new(self.app_db.clone());
        let env = env_builder.instrumented(
            name.to_string(),
            runtime,
            network,
            working_dir.to_string(),
        )?;
        self.instrumented_envs.push(Box::new(env));
        Ok(self)
    }
}

pub struct ComparisonContext<'ctx> {
    app_db: Rc<AppDb>,
    app_config: &'ctx Config,
    env_builder: RuntimeEnvBuilder,
    baseline_env: Option<Box<dyn ReadableEnv + 'ctx>>,
    instrumented_envs: Vec<Box<dyn WriteableEnv + 'ctx>>,
}

impl<'ctx> ComparisonContext<'ctx> {
    /// Creates a new, empty [ComparisonContext].
    pub fn new(config: &'ctx Config, app_db: Rc<AppDb>) -> Self {
        Self {
            env_builder: RuntimeEnvBuilder::new(app_db.clone()),
            app_db,
            baseline_env: None,
            instrumented_envs: Vec::new(),
            app_config: config,
        }
    }

    /// Sets the baseline environment to use for comparison.
    pub fn using_baseline<F>(mut self, f: F) -> Result<Self>
    where
        F: FnOnce(BaselineBuilder<'ctx>) -> Result<BaselineBuilder<'ctx>>,
    {
        let mut builder = BaselineBuilder::new(self.app_db.clone());
        builder = f(builder)?;
        self.baseline_env = builder.baseline_env;
        Ok(self)
    }

    /// Adds a [WriteableEnv] to the instrumentation list for comparison. These
    /// environments will be replayed into and then compared against eachother.
    pub fn instrument_into<F>(mut self, f: F) -> Result<Self>
    where
        F: FnOnce(InstrumentIntoBuilder<'ctx>) -> Result<InstrumentIntoBuilder<'ctx>>,
    {
        let mut builder = InstrumentIntoBuilder::new(self.app_db.clone());
        builder = f(builder)?;
        self.instrumented_envs
            .append(&mut builder.instrumented_envs);
        Ok(self)
    }

    pub fn finish(self) -> Self {
        self
    }

    /// Executes the replay process from the baseline environment into the
    /// environments specified to instrument into.
    pub fn replay<C: ReplayCallbackHandler>(
        mut self,
        opts: &ReplayOpts<C>,
    ) -> Result<ReplayResult> {
        let mut baseline_env_taken = self.baseline_env.take();
        let baseline_env = baseline_env_taken
            .as_mut()
            .ok_or(anyhow!("baseline environment not specified"))?;

        // Open all necessary databases/datastores for the source environment.
        baseline_env.open()?;

        let environments = self.instrumented_envs.iter_mut();
        for target in environments {
            let target_name = &target.name();

            target.open()?;

            let baseline_readable: &dyn ReadableEnv = &**baseline_env as &dyn ReadableEnv;
            let target_writeable: &dyn WriteableEnv = &**target as &dyn WriteableEnv;

            if Self::is_environment_import_needed(baseline_readable, target_writeable)? {
                info!(
                    "[{target_name}] migrating burnstate from '{}'...",
                    baseline_env.name(),
                );
                // Import source burnstate into target environment. This is done due to
                // burnstate being expected to be present during contract evaluation.
                target.import_burnstate(baseline_readable)?;
                info!("finished");

                info!(
                    "[{target_name}] migrating chainstate from '{}'...",
                    baseline_env.name()
                );
                target.import_chainstate(baseline_readable)?;
                info!("finished");
            }

            // Move this into the if-statement above when it works
            info!("[{target_name} preparing to snapshot environment....");
            Self::snapshot_environment(&**target)?;

            // Replay from source into target.
            ChainStateReplayer::replay(&**baseline_env, &mut **target, opts)?;
        }

        todo!()
    }

    fn is_environment_import_needed<Source: ReadableEnv + ?Sized, Target: ReadableEnv + ?Sized>(
        source: &Source,
        target: &Target,
    ) -> Result<bool> {
        info!("checking import data equality between source and target environments...");
        if source.snapshot_count()? != target.snapshot_count()? {
            debug!("environment snapshot counts differ");
            return Ok(true);
        }
        if source.block_commit_count()? != target.block_commit_count()? {
            debug!("block commit counts differ");
            return Ok(true);
        }
        if source.ast_rule_count()? != target.ast_rule_count()? {
            debug!("ast rule counts differ");
            return Ok(true);
        }
        if source.epoch_count()? != target.epoch_count()? {
            debug!("epoch counts differ");
            return Ok(true);
        }
        if source.block_header_count()? != target.block_header_count()? {
            debug!("block header counts differ");
            return Ok(true);
        }

        info!("found no differences between environments; continuing...");
        Ok(false)
    }

    fn snapshot_environment<Target: ReadableEnv + ?Sized>(target: &Target) -> Result<()> {
        let name = target.name();

        // TODO: Load environment from src-target.backup if exists and --reset-env
        // is set.
        let init_chainstate_snapshot_path =
            append_to_path(target.cfg().chainstate_index_db_path(), ".zstd");
        let init_chainstate_snapshot_exists =
            std::fs::metadata(&init_chainstate_snapshot_path).is_ok();
        let init_burnstate_snapshot_path =
            append_to_path(target.cfg().sortition_db_path(), ".zstd");
        let init_burnstate_snapshot_exists =
            std::fs::metadata(&init_burnstate_snapshot_path).is_ok();

        // TODO: Backup environment
        if !init_chainstate_snapshot_exists {
            // Chainstate Index DB
            info!("[{name}] chainstate index snapshot does not exist, creating it...");
            debug!(
                "[{name}] source file: '{:?}'",
                target.cfg().chainstate_index_db_path()
            );
            debug!(
                "[{name}] target file: '{:?}'",
                &init_chainstate_snapshot_path
            );
            debug!("[{name}] opening db file for read");
            let db_file = File::open(target.cfg().chainstate_index_db_path())?;
            let db_reader = BufReader::new(db_file);
            debug!("[{name}] creating target file");
            let file = File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&init_chainstate_snapshot_path)?;
            debug!("[{name}] creating buffered writer for target");
            let file_writer = BufWriter::new(file);
            debug!("[{name}] creating compressed snapshot...");
            zstd::stream::copy_encode(db_reader, file_writer, 5)?;
            debug!("[{name}] finished");
        }

        if !init_burnstate_snapshot_exists && !target.cfg().is_sortition_app_indexed() {
            // Sortition DB
            std::fs::create_dir_all(target.cfg().sortition_dir())?;
            let db_file = File::open(target.cfg().sortition_db_path())?;
            let db_reader = BufReader::new(db_file);
            std::fs::create_dir_all(&init_burnstate_snapshot_path)?;
            let file = File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&init_burnstate_snapshot_path)?;
            let file_writer = BufWriter::new(file);
            zstd::stream::copy_encode(db_reader, file_writer, 5)?;
        }

        ok!()
    }
}

pub enum BlockTransactionContext<'a, 'b> {
    Genesis,
    Regular(RegularBlockTransactionContext<'b, 'a>),
}

pub struct RegularBlockTransactionContext<'b, 'a: 'b> {
    pub stacks_block_id: stacks::StacksBlockId,
    pub clarity_block_conn: stacks::ClarityBlockConnection<'a, 'b>,
    pub clarity_tx_conn: Option<stacks::ClarityTransactionConnection<'a, 'b>>,
}

impl<'a, 'b> RegularBlockTransactionContext<'a, 'b> {
    pub fn new(
        stacks_block_id: stacks::StacksBlockId,
        clarity_block_conn: stacks::ClarityBlockConnection<'a, 'b>,
    ) -> Self {
        Self {
            stacks_block_id,
            clarity_block_conn,
            clarity_tx_conn: None,
        }
    }

    pub fn begin<'c: 'a>(&'c mut self) -> Result<()> {
        self.clarity_tx_conn = Some(self.clarity_block_conn.start_transaction_processing());
        Ok(())
    }
}

/// Represents a Clarity smart contract.
#[derive(Debug)]
pub struct Contract {
    analysis: clarity::ContractAnalysis,
}

impl Contract {
    pub fn new(analysis: clarity::ContractAnalysis) -> Self {
        Self { analysis }
    }

    pub fn contract_analysis(&self) -> &clarity::ContractAnalysis {
        &self.analysis
    }
}

/// Indicates which Clarity runtime should be used for processing transactions.
#[derive(Debug, Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub enum Runtime {
    None = 0,
    Interpreter = 1,
    Wasm = 2,
}

impl From<&Runtime> for i32 {
    fn from(value: &Runtime) -> Self {
        match *value {
            Runtime::None => 0,
            Runtime::Interpreter => 1,
            Runtime::Wasm => 2,
        }
    }
}

impl TryFrom<i32> for Runtime {
    type Error = color_eyre::Report;

    fn try_from(value: i32) -> Result<Self> {
        match value {
            0 => Ok(Runtime::None),
            1 => Ok(Runtime::Interpreter),
            2 => Ok(Runtime::Wasm),
            _ => bail!("Could not convert i32 '{}' to Runtime enum", value),
        }
    }
}

impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runtime::None => write!(f, "None"),
            Runtime::Interpreter => write!(f, "Interpreter"),
            Runtime::Wasm => write!(f, "Wasm"),
        }
    }
}

#[derive(Debug, Clone)]
/// Indicates the type of backing store is used for this environment.
pub enum StoreType {
    /// Uses the standard Stacks-node storage structure and datastores. Use this
    /// option for your source data if it was not explicitly created using this
    /// application.
    StacksNode,
    /// Uses the standard MARF, however Clarity backing stores are instrumented.
    Instrumented,
}

#[derive(Debug, Clone, Copy)]
/// Indicates the network the chain should be configured for, as well as the
/// chain id.
pub enum Network {
    Mainnet(u32),
    Testnet(u32),
}

impl Network {
    pub fn is_mainnet(&self) -> bool {
        matches!(self, Network::Mainnet(_))
    }

    pub fn chain_id(&self) -> u32 {
        match self {
            Network::Mainnet(i) => *i,
            Network::Testnet(i) => *i,
        }
    }

    pub fn mainnet(chain_id: u32) -> Network {
        Network::Mainnet(chain_id)
    }

    pub fn testnet(chain_id: u32) -> Network {
        Network::Testnet(chain_id)
    }
}
