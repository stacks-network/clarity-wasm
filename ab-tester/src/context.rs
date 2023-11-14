use std::rc::Rc;

use color_eyre::{
    eyre::{anyhow, bail},
    Result,
};
use log::*;

use crate::{clarity, context::replay::ChainStateReplayer, db::appdb::AppDb};

pub mod blocks;
mod boot_data;
pub mod callbacks;
pub mod environments;
mod marf;
pub mod replay;

use self::environments::{RuntimeEnvBuilder, RuntimeEnvContext, RuntimeEnv, RuntimeEnvContextMut};
use self::replay::{ReplayOpts, ReplayResult};
pub use blocks::{Block, BlockCursor};

pub struct BaselineBuilder(ComparisonContext);

impl BaselineBuilder {
    pub fn stacks_node(
        mut self, 
        name: String, 
        node_dir: String
    ) -> Result<ComparisonContext> {
        let env = self.0.env_builder.stacks_node(name, node_dir)?;
        let env_ctx = RuntimeEnvContext::new(env);
        self.0.baseline_env = Some(env_ctx);
        Ok(self.0)
    }
}

pub struct InstrumentIntoBuilder<'a>(&'a mut ComparisonContext);

impl<'a> InstrumentIntoBuilder<'a> {
    pub fn instrumented(
        &mut self, 
        name: String,
        runtime: Runtime,
        network: Network,
        working_dir: String
    ) -> Result<&'_ mut ComparisonContext> {
        let env = self.0.env_builder.instrumented(name, runtime, network, working_dir)?;
        let env_ctx = RuntimeEnvContextMut::new(env);
        self.0.instrumented_envs.push(env_ctx);
        Ok(self.0)
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
            app_db: Rc::clone(&app_db),
            env_builder: RuntimeEnvBuilder::new(Rc::clone(&app_db)),
            baseline_env: None,
            instrumented_envs: Vec::new(),
        }
    }

    /// Sets the baseline environment to use for comparison.
    pub fn using_baseline(
        self, 
        f: impl FnOnce(BaselineBuilder) -> Result<Self>
    ) -> Result<Self> {
        let builder = BaselineBuilder(self);
        let ctx = f(builder)?;
        Ok(ctx)
    }

    /// Adds a [WriteableEnv] to the instrumentation list for comparison. These
    /// environments will be replayed into and then compared against eachother.
    pub fn instrument_into(
        &mut self, 
        f: impl FnOnce(InstrumentIntoBuilder) -> Result<Self>
    ) -> Result<Self> {
        let builder = InstrumentIntoBuilder(self);
        let ctx = f(builder)?;
        Ok(ctx)
    }

    /// Executes the replay process from the baseline environment into the
    /// environments specified to instrument into.
    pub fn replay(&mut self, opts: &ReplayOpts) -> Result<ReplayResult> {
        let baseline_env = self
            .baseline_env
            .as_mut()
            .ok_or(anyhow!("baseline environment has need been specified"))?;

        baseline_env.open()?;

        for target in self.instrumented_envs.iter_mut() {
            ChainStateReplayer::replay(baseline_env, target, opts)?;
        }

        todo!()
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
struct TestEnvPaths {
    working_dir: String,

    index_db_path: String,
    sortition_db_path: String,
    blocks_dir: String,
    chainstate_path: String,
    clarity_db_path: String,
}

impl TestEnvPaths {
    pub fn new(working_dir: &str) -> Self {
        Self {
            working_dir: working_dir.to_string(),

            index_db_path: format!("{}/chainstate/vm/index.sqlite", working_dir),
            sortition_db_path: format!("{}/burnchain/sortition", working_dir),
            blocks_dir: format!("{}/chainstate/blocks", working_dir),
            chainstate_path: format!("{}/chainstate", working_dir),
            clarity_db_path: format!("{}/chainstate/vm/clarity/marf.sqlite", working_dir),
        }
    }

    pub fn print(&self, env_name: &str) {
        info!("[{env_name}] using working dir: {}", self.working_dir);
        debug!("[{env_name}] index db: {}", self.index_db_path);
        debug!("[{env_name}] sortition db: {}", self.sortition_db_path);
        debug!("[{env_name}] clarity db: {}", self.clarity_db_path);
        debug!("[{env_name}] blocks dir: {}", self.blocks_dir);
        debug!("[{env_name}] chainstate dir: {}", self.chainstate_path);
    }
}
