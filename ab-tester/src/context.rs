use color_eyre::{
    eyre::{anyhow, bail, ensure},
    Result,
};
use log::*;

use crate::{clarity, db::appdb::AppDb, errors::AppError, ok};

pub mod blocks;
mod boot_data;
pub mod environments;
mod marf;
pub mod callbacks;

use self::environments::{ReadableEnv, WriteableEnv};
pub use blocks::{Block, BlockCursor};

pub struct ComparisonContext<'a> {
    app_db: &'a AppDb,
    baseline_env: Option<&'a mut dyn ReadableEnv<'a>>,
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

    pub fn using_baseline(&'a mut self, env: &'a mut impl ReadableEnv<'a>) -> &'a mut Self {
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

    pub fn replay(&mut self, opts: Option<ReplayOpts>) -> Result<ReplayResult> {
        let baseline_env = self
            .baseline_env
            .as_mut()
            .ok_or(anyhow!("baseline environment has need been specified"))?;

        baseline_env.open()?;

        for env in &mut self.instrumented_envs {
            info!(
                "replaying from '{}' into '{}'...",
                baseline_env.name(),
                env.name()
            );
            ChainStateReplayer::replay(*baseline_env, *env, opts)?;
        }

        todo!()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReplayOpts {
    pub from_height: Option<u32>,
    pub to_height: Option<u32>,
    pub max_blocks: Option<u32>,
}

impl ReplayOpts {
    pub fn assert_max_processed_block_count(&self, processed_block_count: u32) -> Result<()> {
        if let Some(max_blocks) = self.max_blocks {
            ensure!(
                processed_block_count < max_blocks,
                AppError::Graceful("number of blocks processed has reached the specified maximum")
            );
        }

        ok!()
    }

    pub fn assert_block_height_under_max_height(&self, block_height: u32) -> Result<()> {
        if let Some(to_height) = self.to_height {
            ensure!(
                block_height <= to_height,
                AppError::Graceful(
                    "block height has reached the specified maximum block height (to-height)"
                )
            )
        }

        ok!()
    }
}

pub struct ChainStateReplayer {}

impl ChainStateReplayer {
    pub fn replay<'a>(
        source: &'_ mut dyn ReadableEnv<'a>,
        target: &'_ mut dyn WriteableEnv<'a>,
        opts: Option<ReplayOpts>,
    ) -> Result<()> {
        let opts = opts.unwrap_or(ReplayOpts::default());

        info!(
            "aggregating contract calls starting at block height {}...",
            opts.from_height.unwrap_or(0)
        );

        let mut processed_block_count = 0;

        for block in source.blocks()?.into_iter() {
            let (header, stacks_block) = match &block {
                Block::Boot(header) => {
                    // We can't process the boot block, so skip it.
                    info!("boot block - skipping '{:?}'", header.index_block_hash);
                    continue;
                }
                Block::Genesis(header) => {
                    // We can't process genesis (doesn't exist in chainstate), so skip it.
                    //info!("genesis block - skipping '{:?}'", gen.index_block_hash);
                    //continue;
                    info!("genesis block");
                    (header, None)
                }
                Block::Regular(header, block) => (header, Some(block)),
            };

            // Ensure that we've reached the specified block-height before beginning
            // processing.
            if header.block_height() < opts.from_height.unwrap_or(0) {
                continue;
            }

            // Ensure that we haven't exceeded the specified max-blocks for processing.
            opts.assert_max_processed_block_count(processed_block_count)?;

            // Ensure that we haven't reached the specified max block-height for processing.
            opts.assert_block_height_under_max_height(header.block_height())?;

            debug!(
                "processing block #{} ({})",
                header.block_height(),
                &hex::encode(&header.index_block_hash)
            );
            /*replay_env_mut.block_begin(&block, |_ctx| {
                info!("processing block!");
                ok!()
            })?;*/

            processed_block_count += 1;
            continue;

            /*let block_id = header.stacks_block_id()?;

            for tx in stacks_block.unwrap().txs.iter() {
                info!("processing tx: {}", tx.txid());

                let origin_principal = StandardPrincipalData::from(tx.origin_address());

                #[allow(clippy::single_match)]
                match &tx.payload {
                    TransactionPayload::SmartContract(contract, _) => {
                        let contract_id =
                            QualifiedContractIdentifier::new(origin_principal, contract.name.clone());

                        if let Some(entry) = contracts.get(&contract_id) {
                            warn!(
                                "duplicate: {}, first block={}, second block={}",
                                contract_id, entry, &block_id
                            );
                        } else {
                            contracts.insert(contract_id, block_id);
                        }
                    }
                    _ => {}
                }
            }*/
        }

        info!("blocks processed: {processed_block_count}");

        ok!()
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
