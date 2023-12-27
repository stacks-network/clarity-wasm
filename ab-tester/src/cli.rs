pub mod commands;
mod console;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ArgGroup, ValueEnum};
use clap_verbosity_flag::Verbosity;
use color_eyre::eyre::{bail, Result};

use crate::context::callbacks::ReplayCallbackHandler;
use crate::context::replay::ReplayOpts;
use crate::context::Runtime;
use crate::ok;

/// Our CLI entrypoint.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbosity: Verbosity,

    #[arg(
        long = "config",
        short = 'c',
        default_value = "./config.toml",
        value_name = "CONFIG FILE",
        help = "Use the specified configuration file.",
        global = true
    )]
    pub config: PathBuf,

    #[arg(
        long = "sql-trace", 
        short = None,
        default_value = None,
        help = "Enable SQL query tracing",
        global = true
    )]
    pub sql_trace: bool,

    #[arg(
        short = None,
        long = "disable-stacks-logging",
        help = "Disables logging of Stacks node output to the console.",
        default_value = "false",
        global = true,
    )]
    pub disable_stacks_logging: bool,
}

impl Cli {
    /// Asserts that the provided configuration file exists and if exists
    /// returns the path as a String.
    pub fn config_file_path(&self) -> Result<String> {
        if !self.config.exists() {
            bail!(
                "specified configuration file does not exist: '{}'",
                self.config.display()
            );
        }

        Ok(self.config.display().to_string())
    }

    /// Performs validation of the parsed command line arguments.
    pub fn validate(self) -> Result<Self> {
        match &self.command {
            Commands::Data(args) => DataArgs::validate(args)?,
            Commands::Tui(args) => TuiArgs::validate(args)?,
            Commands::Env(_args) => {}
        }

        Ok(self)
    }
}

/// Enum which defines our root subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    Tui(TuiArgs),
    Data(DataArgs),
    Env(EnvArgs),
}

/// Commands for managing runtime environments for this tool.
#[derive(Debug, Args)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub commands: EnvSubCommands,
}

#[derive(Debug, Subcommand)]
pub enum EnvSubCommands {
    New(NewEnvArgs),
    List(ListEnvArgs),
    Snapshot(SnapshotEnvArgs)
}

#[derive(Debug, Subcommand)]
pub enum NewEnvSubCommands {
    /// Opens an existing Stacks node's data directory. This environment-type is
    /// read-only and can only be used as a source environment.
    StacksNode(NewStacksNodeEnvArgs),
    /// Opens an existing or creates a new instrumented environment which can
    /// be used for comparisons. This environment type can be used both as a source
    /// and target for comparisons.
    Instrumented(NewInstrumentedEnvArgs),
    /// Opens an existing or creates a new network-synced environment. This
    /// environment-type is read-only and can only be used as a source environment.
    Network(NewNetworkEnvArgs),
}

#[derive(Debug, Clone, Args)]
pub struct NewStacksNodeEnvArgs {
    #[arg(
        short = 'p',
        long = "path",
        help = "The Stacks node's root path, e.g. `xx/mainnet/`.",
        required = true
    )]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct NewInstrumentedEnvArgs {
    #[arg(
        short = 'r',
        long = "runtime",
        help = "The Clarity runtime to be used for this environment.",
        required = true
    )]
    pub runtime: Runtime,

    #[arg(
        short = None,
        long = "read-only",
        help = "Whether or not this environment should be read-only, i.e. can only be used as a source",
        required = false,
        default_value = "true"
    )]
    pub is_read_only: bool,

    #[arg(
        short = None,
        long = "network",
        help = "The network which the environment runs on.",
        required = true
    )]
    pub network: NetworkChoice,

    #[arg(
        short = None,
        long = "chain-id",
        help = "The chain-id which the environment runs on.",
        required = false,
        default_value = "1"
    )]
    pub chain_id: u32,

    #[arg(
        short = 'p',
        long = "path",
        help = "The working directory for the environment where chainstate, burnstate and blocks will be stored.",
        required = false,
        default_value = None
    )]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct NewNetworkEnvArgs {
    pub peer_host: String,
    pub peer_port: u16,
    pub peer_key: String,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum NetworkChoice {
    Mainnet,
    Testnet
}

#[derive(Debug, Args)]
pub struct NewEnvArgs {
    #[command(subcommand)]
    pub commands: NewEnvSubCommands,

    #[arg(
        short = 'n',
        long = "name",
        help = "The name of the environment.",
        required = true
    )]
    pub name: String,
}

#[derive(Debug, Args)]
#[clap(group(
    ArgGroup::new("env")
        .required(true)
        .multiple(false)
        .args(&["env_name", "env_id"]),
))]

pub struct SnapshotEnvArgs {
    #[arg(
        long = "env-name"
    )]
    pub env_name: Option<String>,
    #[arg(
        long = "env-id"
    )]
    pub env_id: Option<i32>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum EnvType {
    StacksNode,
    Instrumented,
    Network,
}

#[derive(Debug, Args)]
pub struct ListEnvArgs {}

/// Run the interactive terminal interface.
#[derive(Debug, Args)]
pub struct TuiArgs {
    #[arg(
        long = "theme",
        help = "Sets the color theme for the console.",
        default_value = None
    )]
    pub theme: Option<String>,
}

impl TuiArgs {
    pub fn validate(_args: &Self) -> Result<()> {
        ok!()
    }
}

/// Commands for data-processing using the command line.
#[derive(Debug, Args)]
pub struct DataArgs {
    #[arg(
        short = 'f',
        long = "from-height",
        default_value = "0",
        help = "The block height at which to begin processing."
    )]
    pub from_height: u32,

    #[arg(
        short = 't',
        long = "to-height",
        default_value = None,
        help = "The block height at which to stop processing (inclusive)."
    )]
    pub to_height: Option<u32>,

    #[arg(
        short = 'l',
        long = "max-blocks",
        help = "Stops processing after the specified number of blocks."
    )]
    pub max_block_count: Option<u32>,

    #[arg(
        short = 'i',
        long = "contract-id",
        help = "Filter all processing to only the specified qualified contract id."
    )]
    pub contract_id: Option<String>,

    #[arg(
        short = 'r',
        long = "snapshot-restore",
        help = "Restores the target environment to the specified snapshot before processing."
    )]
    pub snapshot_restore: bool,

    #[arg(
        short = None,
        long = "resume",
        help = "Resumes an import from the last processed block height",
        default_value = "true"
    )]
    pub resume: bool
}

impl<C: ReplayCallbackHandler + Default> From<DataArgs> for ReplayOpts<C> {
    fn from(value: DataArgs) -> Self {
        ReplayOpts {
            from_height: Some(value.from_height),
            to_height: value.to_height,
            max_blocks: value.max_block_count,
            callbacks: C::default(),
            working_dir: Default::default(),
            snapshot_restore: value.snapshot_restore
        }
    }
}

/// Implements helper functions for [DataArgs].
impl DataArgs {
    pub fn validate(_args: &Self) -> Result<()> {
        ok!()
    }
}
