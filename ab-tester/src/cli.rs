pub mod commands;

use std::path::PathBuf;

use anyhow::{anyhow, ensure, Ok, Result};
use clap::{Args, Parser, Subcommand};

use crate::errors::AppError;
use crate::ok;

/// Our CLI entrypoint.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Enum which defines our root subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    Tui(TuiArgs),
    Data(DataArgs),
}

/// Arguments for the `tui` subcommand, used together with the [commands::tui]
/// command implementation.
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

/// Arguments for the `data` subcommand, used together with the [commands::data]
/// command implementation.
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

/// Implements helper functions for [DataArgs].
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