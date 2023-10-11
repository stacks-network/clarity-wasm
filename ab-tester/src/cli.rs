use std::path::PathBuf;

use anyhow::{anyhow, bail, ensure, Error, Ok, Result};
use clap::{Args, Parser, Subcommand};
use log::info;

use crate::errors::{self, AppError};
use crate::ok;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Tui(TuiArgs),
    Data(DataArgs),
}

#[derive(Debug, Args)]
pub struct TuiArgs {
    #[arg(
        long = "config",
        default_value = "./config.toml",
        value_name = "CONFIG FILE",
        help = "Use the specified configuration file."
    )]
    pub config: Option<PathBuf>,
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
}

impl DataArgs {
    pub fn assert_max_processed_block_count(&self, processed_block_count: u32) -> Result<()> {
        if let Some(max_blocks) = self.max_block_count {
            ensure!(
                processed_block_count < max_blocks,
                AppError::Graceful(anyhow!(
                    "number of blocks processed has reached the specified maximum"
                ))
            );
        }

        ok!()
    }

    pub fn assert_block_height_under_max_height(&self, block_height: u32) -> Result<()> {
        if let Some(to_height) = self.to_height {
            ensure!(
                block_height < to_height,
                AppError::Graceful(anyhow!(
                    "block height has reached the specified maximum block height (to-height)"
                ))
            )
        }

        ok!()
    }
}

/*// Check if we've reached the specified max blocks processed count.
if let Some(max_blocks) = data_args.max_block_count {
    if block_count >= max_blocks {
        info!("reached max block count ({}), exiting.", max_blocks);
        exit(0);
    }
}

if let Some(to_height) = data_args.to_height {
    if block_header.block_height > to_height - 1 {
        info!("reached block height limit ({}), exiting.", block_header.block_height);
        exit(0);
    }
}


if block_header.block_height < data_args.from_height {
    continue;
} */
