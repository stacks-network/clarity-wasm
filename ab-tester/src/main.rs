// TODO: Remove this
#![allow(dead_code)]

mod cli;
mod config;
mod context;
mod errors;
#[macro_use]
mod macros;
mod clarity;
mod db;
mod environments;
mod runtime;
mod stacks;
mod types;
mod utils;

use std::process::exit;

use clap::Parser;
use cli::*;
use color_eyre::eyre::{bail, Result};
use config::Config;
use console::Color;
use diesel::connection::SimpleConnection;
use diesel::{Connection, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::*;

use crate::errors::AppError;

// Embed our database migrations at compile-time so that they can easily be
// run at applicaton execution without needing external SQL files.
pub const DB_MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error reporting.
    color_eyre::install()?;

    // Parse & validate command line arguments.
    let cli = Cli::parse().validate()?;

    // Initialize logging.
    configure_logging(&cli);

    // Load the application configuration file. If the `--config` CLI parameter
    // has been provided, attempt to use the provided path, otherwise use the
    // default `config.toml`.
    let config = Config::load(&cli.config_file_path()?)?;

    // Apply any pending database migrations. If the application database doesn't
    // exist it will be created.
    apply_db_migrations(&config)?;

    // Execute the given command with args.
    let _ = match cli.command {
        Commands::Tui(args) => commands::console::exec(&config, args).await,
        Commands::Data(args) => commands::data::exec(config.clone(), args).await,
        Commands::Env(args) => commands::env::exec(&config, args).await,
    }
    .map_err(|err| match err.downcast_ref() {
        Some(AppError::Graceful(graceful)) => {
            println!(
                "{} {:?}",
                console::style("finished:").bold().fg(Color::Green),
                graceful
            );
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

    app_db.batch_execute("
        PRAGMA journal_mode = WAL;          -- better write-concurrency
        PRAGMA synchronous = NORMAL;        -- fsync only in critical moments
        PRAGMA wal_autocheckpoint = 1000;   -- write WAL changes back every 1000 pages, for an in average 1MB WAL file. May affect readers if number is increased
        PRAGMA wal_checkpoint(TRUNCATE);    -- free some space by truncating possibly massive WAL files from the last run.
        PRAGMA busy_timeout = 250;          -- sleep if the database is busy
        PRAGMA foreign_keys = ON;           -- enforce foreign keys
    ")?;

    let has_pending_migrations =
        MigrationHarness::has_pending_migration(&mut app_db, DB_MIGRATIONS)
            .or_else(|e| bail!("failed to determine database migration state: {:?}", e))?;

    if has_pending_migrations {
        info!("there are pending database migrations - updating the database");

        MigrationHarness::run_pending_migrations(&mut app_db, DB_MIGRATIONS)
            .or_else(|e| bail!("failed to run database migrations: {:?}", e))?;

        info!("database migrations have been applied successfully");
    }

    ok!()
}

fn configure_logging(cli: &Cli) {
    // Initialize logging.
    env_logger::Builder::new()
        .filter_level(cli.verbosity.log_level_filter())
        .init();

    if cli.sql_trace {
        std::env::set_var("sql_trace", "1");
    }

    if let Some(level) = cli.verbosity.log_level() {
        match level {
            Level::Trace => {
                std::env::set_var("BLOCKSTACK_TRACE", "1");
                std::env::set_var("STACKS_LOG_TRACE", "1");
            }
            Level::Debug => {
                std::env::set_var("BLOCKSTACK_DEBUG", "1");
                std::env::set_var("STACKS_LOG_DEBUG", "1");
            }
            Level::Info => {
                std::env::set_var("BLOCKSTACK_INFO", "1");
                std::env::set_var("STACKS_LOG_INFO", "1");
            }
            Level::Warn => {
                std::env::set_var("BLOCKSTACK_WARN", "1");
                std::env::set_var("STACKS_LOG_WARN", "1");
            }
            Level::Error => {
                std::env::set_var("BLOCKSTACK_ERROR", "1");
                std::env::set_var("STACKS_LOG_ERROR", "1");
            }
        }
    }
}
