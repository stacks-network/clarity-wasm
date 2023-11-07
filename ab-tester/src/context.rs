use std::{fmt::Write, marker::PhantomData};

use color_eyre::{
    eyre::{anyhow, bail},
    Result,
};
use log::*;

use crate::{clarity, db::appdb::AppDb, ok};

pub mod blocks;
mod boot_data;
pub mod environments;
mod marf;

pub use self::environments::TestEnv;
use self::environments::{ReadableEnv, RuntimeEnv, WriteableEnv};
pub use blocks::{Block, BlockCursor};

pub struct ComparisonContext<'a> {
    app_db: &'a AppDb,
    baseline_env: Option<&'a dyn ReadableEnv<'a>>,
    instrumented_envs: Vec<&'a mut dyn WriteableEnv<'a>>,
}

impl<'a> ComparisonContext<'a> {
    pub fn new(app_db: &'a AppDb) -> Self {
        Self {
            app_db,
            baseline_env: None,
            instrumented_envs: Vec::new(),
        }
    }

    pub fn using_baseline(&'a mut self, env: &'a impl ReadableEnv<'a>) -> &'a mut Self {
        self.baseline_env = Some(env);
        self
    }

    pub fn instrument_into<'b: 'a>(
        &'a mut self,
        env: &'b mut impl WriteableEnv<'a>,
    ) -> &'a mut Self {
        self.instrumented_envs.push(env);
        self
    }

    pub fn replay(&mut self) -> Result<ReplayResult> {
        let baseline_env = self
            .baseline_env
            .ok_or(anyhow!("baseline environment has not been set"))?;

        for env in &mut self.instrumented_envs {
            info!(
                "replaying from '{}' into '{}'...",
                baseline_env.name(),
                env.name()
            );
            ChainStateReplayer::replay(self.baseline_env.unwrap(), *env)?;
        }

        todo!()
    }
}

pub struct ChainStateReplayer {}

impl ChainStateReplayer {
    pub fn replay<'a>(
        source: &'_ dyn ReadableEnv<'a>,
        target: &'_ mut dyn WriteableEnv<'a>,
    ) -> Result<()> {
        todo!()
    }
}

pub struct ReplayResult {}

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
#[derive(Debug)]
pub enum Runtime {
    Interpreter = 1,
    Wasm = 2,
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
