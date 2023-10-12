mod cli;
mod config;
mod context;
mod errors;
mod model;
mod schema;
#[macro_use]
mod macros;

use anyhow::{Result, bail};
use clap::Parser;
use cli::*;
use config::Config;
use diesel::{SqliteConnection, Connection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::*;
use std::process::exit;

use crate::errors::AppError;

pub const DB_MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn main() -> Result<()> {
    // Initialize logging.
    env_logger::init();

    // Parse & validate command line arguments.
    let cli = Cli::parse().validate()?;

    // Load the application configuration file. If the `--config` CLI parameter
    // has been provided, attempt to use the provided path, otherwise use the
    // default `config.toml`.
    let config = Config::load(&cli.config_file_path()?)?;

    // Apply any pending database migrations. If the application database doesn't
    // exist it will be created.
    apply_db_migrations(&config)?;

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
            error!("the application encountered a fatal error: {err:?}");
            exit(2)
        }
    });

    ok!()
}

/// Applies any pending application database migrations. Initializes the
/// database if it does not already exist.
fn apply_db_migrations(config: &Config) -> Result<()> {
    let mut app_db = SqliteConnection::establish(&config.app.db_path)?;
    let has_pending_migrations = MigrationHarness::has_pending_migration(&mut app_db, DB_MIGRATIONS)
        .or_else(|e| bail!("failed to determine database migration state: {:?}", e))?;
    
    if has_pending_migrations {
        info!("there are pending database migrations - updating the database");

        MigrationHarness::run_pending_migrations(&mut app_db, DB_MIGRATIONS)
            .or_else(|e| bail!("failed to run database migrations: {:?}", e))?;

        info!("database migrations have been applied successfully");
    }

    ok!()
}