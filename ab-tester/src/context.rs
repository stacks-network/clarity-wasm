use std::rc::Rc;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use log::*;

use crate::context::replay::ChainStateReplayer;
use crate::db::appdb::AppDb;
use crate::{clarity, stacks};

pub mod blocks;
pub mod boot_data;
pub mod callbacks;
pub mod replay;

pub use blocks::{Block, BlockCursor};

use crate::environments::{RuntimeEnvBuilder, RuntimeEnvContext, RuntimeEnvContextMut};
use self::replay::{ReplayOpts, ReplayResult};

pub struct BaselineBuilder(ComparisonContext);

impl BaselineBuilder {
    pub fn stacks_node(mut self, name: &str, node_dir: &str) -> Result<ComparisonContext> {
        let env = self
            .0
            .env_builder
            .stacks_node(name.to_string(), node_dir.to_string())?;
        let env_ctx = RuntimeEnvContext::new(env);
        self.0.baseline_env = Some(env_ctx);
        Ok(self.0)
    }
}

pub struct InstrumentIntoBuilder<'a>(&'a mut ComparisonContext);

impl<'a> InstrumentIntoBuilder<'a> {
    pub fn instrumented(
        self,
        name: &str,
        runtime: Runtime,
        network: Network,
        working_dir: &str,
    ) -> Result<InstrumentIntoBuilder<'a>> {
        let env = self.0.env_builder.instrumented(
            name.to_string(),
            runtime,
            network,
            working_dir.to_string(),
        )?;
        let env_ctx = RuntimeEnvContextMut::new(env);
        self.0.instrumented_envs.push(env_ctx);
        Ok(self)
    }
}

pub struct ComparisonContext {
    app_db: Rc<AppDb>,
    env_builder: RuntimeEnvBuilder,
    baseline_env: Option<RuntimeEnvContext>,
    instrumented_envs: Vec<RuntimeEnvContextMut>,
}

impl ComparisonContext {
    /// Creates a new, empty [ComparisonContext].
    pub fn new(app_db: Rc<AppDb>) -> Self {
        Self {
            env_builder: RuntimeEnvBuilder::new(app_db.clone()),
            app_db,
            baseline_env: None,
            instrumented_envs: Vec::new(),
        }
    }

    /// Sets the baseline environment to use for comparison.
    pub fn using_baseline(self, f: impl FnOnce(BaselineBuilder) -> Result<Self>) -> Result<Self> {
        let builder = BaselineBuilder(self);
        let ctx = f(builder)?;
        Ok(ctx)
    }

    /// Adds a [WriteableEnv] to the instrumentation list for comparison. These
    /// environments will be replayed into and then compared against eachother.
    pub fn instrument_into(
        &mut self,
        f: impl FnOnce(InstrumentIntoBuilder) -> Result<InstrumentIntoBuilder>,
    ) -> Result<&mut Self> {
        let builder = InstrumentIntoBuilder(self);
        f(builder)?;
        Ok(self)
    }

    /// Executes the replay process from the baseline environment into the
    /// environments specified to instrument into.
    pub fn replay(&mut self, opts: &ReplayOpts) -> Result<ReplayResult> {
        let mut baseline_env_taken = self.baseline_env.take();
        let baseline_env = baseline_env_taken
            .as_mut()
            .ok_or(anyhow!("baseline environment not specified"))?;

        // Open all necessary databases/datastores for the source environment.
        baseline_env.open()?;

        let environments = self.instrumented_envs.iter_mut();
        for target in environments {
            target.open()?;

            info!(
                "migrating burnstate from '{}' to '{}'...",
                baseline_env.name(),
                target.name()
            );

            // Import source burnstate into target environment. This is done due to
            // burnstate being expected to be present during contract evaluation.
            target.import_burnstate(baseline_env.as_readable_env())?;
            info!("finished - proceeding with replay");

            // Replay from source into target.
            ChainStateReplayer::replay(baseline_env, target, opts)?;
        }

        todo!()
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

/// Helper struct to carry all of the different paths involved in chainstate
/// and sortition.
#[derive(Debug, Clone)]
pub struct StacksEnvPaths {
    pub working_dir: String,

    pub index_db_path: String,
    pub sortition_dir: String,
    pub sortition_db_path: String,
    pub blocks_dir: String,
    pub chainstate_dir: String,
    pub clarity_db_path: String,
}

impl StacksEnvPaths {
    /// Creates a new instance of [StacksEnvPaths] from the provided base
    /// `working_dir`. This will populate all of the relevent paths needed for
    /// this application.
    pub fn new(working_dir: &str) -> Self {
        Self {
            working_dir: working_dir.to_string(),

            index_db_path: format!("{}/chainstate/vm/index.sqlite", working_dir),
            sortition_dir: format!("{}/burnchain/sortition", working_dir),
            sortition_db_path: format!("{}/burnchain/sortition/marf.sqlite", working_dir),
            blocks_dir: format!("{}/chainstate/blocks", working_dir),
            chainstate_dir: format!("{}/chainstate", working_dir),
            clarity_db_path: format!("{}/chainstate/vm/clarity/marf.sqlite", working_dir),
        }
    }

    /// Prints information about the paths.
    pub fn print(&self, env_name: &str) {
        info!("[{env_name}] using working dir: {}", self.working_dir);
        debug!("[{env_name}] index db: {}", self.index_db_path);
        debug!("[{env_name}] sortition dir: {}", self.sortition_dir);
        debug!("[{env_name}] sortition db: {}", self.sortition_db_path);
        debug!("[{env_name}] clarity db: {}", self.clarity_db_path);
        debug!("[{env_name}] blocks dir: {}", self.blocks_dir);
        debug!("[{env_name}] chainstate dir: {}", self.chainstate_dir);
    }
}
