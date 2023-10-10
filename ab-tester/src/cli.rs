use std::path::PathBuf;

use clap::{Parser, Subcommand, Args};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Tui(TuiArgs),
    Data(DataArgs)
}

#[derive(Debug, Args)]
pub struct TuiArgs {
    #[arg(
        long = "config", 
        default_value = "./config.toml", 
        value_name = "CONFIG FILE",
        help = "Use the specified configuration file."
    )]
    pub config: Option<PathBuf>
}

#[derive(Debug, Args)]
pub struct DataArgs {
    #[arg(
        long = "config", 
        default_value = "./config.toml", 
        value_name = "CONFIG FILE",
        help = "Use the specified configuration file."
    )]
    pub config: Option<PathBuf>,

    #[arg(
        short = None, 
        long = "from-height", 
        default_value = "0", 
        help = "The block height at which to begin processing."
    )]
    pub from_height: i32,

    #[arg(short, long)]
    pub to_height: Option<i32>,

    #[arg(
        short = None, 
        long = "max-blocks",
        help = "Stops processing after the specified number of blocks."
    )]
    pub max_block_count: Option<i32>,

    #[arg(
        short = None,
        long = "contract-id",
        help = "Filter all processing to only the specified qualified contract id."
    )]
    pub contract_id: Option<String>
}