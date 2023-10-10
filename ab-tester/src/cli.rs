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
    #[arg(long, default_value = "./config.toml", value_name = "CONFIG FILE")]
    pub config: Option<PathBuf>
}

#[derive(Debug, Args)]
pub struct DataArgs {
    #[arg(long, default_value = "./config.toml", value_name = "CONFIG FILE")]
    pub config: Option<PathBuf>,

    #[arg(short, long, default_value = "0")]
    pub from_height: i32,

    #[arg(short, long)]
    pub to_height: Option<i32>,

    #[arg(short, long)]
    pub contract_id: Option<String>
}