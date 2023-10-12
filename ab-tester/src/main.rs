mod cli;
mod config;
mod context;
mod errors;
mod model;
mod schema;
#[macro_use]
mod macros;

use anyhow::Result;
use clap::Parser;
use cli::*;
use log::*;
use std::process::exit;

use crate::errors::AppError;

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("cli: {:?}", cli);

    // Initialize logging.
    env_logger::init();

    // Load application configuration file.
    let config_file = &std::env::var("CONFIG_FILE")?;
    let config = crate::config::Config::load(config_file)?;

    // Execute the given command with args.
    let _ = match cli.command {
        Commands::Tui(args) => commands::console::exec(&config, args),
        Commands::Data(args) => commands::data::exec(&config, args),
    }
    .map_err(|err| match err.downcast_ref() {
        Some(AppError::Graceful(graceful)) => {
            info!("terminating gracefully: {graceful:?}");
            exit(0)
        }
        _ => {
            error!("encountered a fatal error: {err:?}");
            exit(2)
        }
    });

    ok!()
}
