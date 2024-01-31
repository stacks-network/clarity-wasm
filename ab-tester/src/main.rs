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
mod telemetry;

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
use tracing::{field, Instrument, Subscriber};
use tracing_subscriber::filter::DynFilterFn;
use tracing_subscriber::fmt::time;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};
use tracing_subscriber::{layer::SubscriberExt, fmt::format::*};
use tracing_tree::HierarchicalLayer;

use crate::errors::AppError;
use crate::telemetry::{ClarityTracingLayer, PrintTreeLayer};

// Embed our database migrations at compile-time so that they can easily be
// run at applicaton execution without needing external SQL files.
pub const DB_MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() -> Result<()> {


    tracing_subscriber::registry()
        .with(ClarityTracingLayer::default())
        .with(PrintTreeLayer::default())
        .init();

    tracing::info!(hello = "world", "no span");
    let outer_span = tracing::info_span!("outer", level = 0, parting = field::Empty);
    let _outer_entered = outer_span.enter();
    {
        let inner_span = tracing::debug_span!("inner", level = 1);
        let _inner_entered = inner_span.enter();
        outer_span.record("parting", "goodbye, world!");
    }

    tracing::info!(a_bool = true, answer = 42, message = "first example");
    drop(_outer_entered);
    exit(0);

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
    if cli.sql_trace {
        std::env::set_var("sql_trace", "1");
    }

    if cli.enable_stacks_logging {
        if let Some(level) = cli.verbosity.log_level() {
            match level {
                Level::Trace => {
                    std::env::set_var("RUST_LOG", "trace");
                    std::env::set_var("BLOCKSTACK_TRACE", "1");
                    std::env::set_var("STACKS_LOG_TRACE", "1");
                }
                Level::Debug => {
                    std::env::set_var("RUST_LOG", "debug");
                    std::env::set_var("BLOCKSTACK_DEBUG", "1");
                    std::env::set_var("STACKS_LOG_DEBUG", "1");
                }
                Level::Info => {
                    std::env::set_var("RUST_LOG", "info");
                    std::env::set_var("BLOCKSTACK_INFO", "1");
                    std::env::set_var("STACKS_LOG_INFO", "1");
                }
                Level::Warn => {
                    std::env::set_var("RUST_LOG", "warn");
                    std::env::set_var("BLOCKSTACK_WARN", "1");
                    std::env::set_var("STACKS_LOG_WARN", "1");
                }
                Level::Error => {
                    std::env::set_var("RUST_LOG", "error");
                    std::env::set_var("BLOCKSTACK_ERROR", "1");
                    std::env::set_var("STACKS_LOG_ERROR", "1");
                }
            }
        }
    }
        
    // Only enable spans or events within a span named "interesting_span".
    /*let my_filter = DynFilterFn::new(|metadata, cx| {
        return true;
        
        if metadata.fields().iter().any(|f| f.name() == "execute_contract") {
            return true;
        }
        false
    });*/

    /*tracing_span_tree::span_tree()
        .aggregate(false)
        .enable();*/

    /*let subscriber = Registry::default()
        .with(HierarchicalLayer::new(2)
            .with_indent_lines(true)
            .with_span_modes(true)
            .with_bracketed_fields(true)
            .with_timer(tracing_tree::time::LocalDateTime)
            .
            .with_higher_precision(true)
        );
    tracing::subscriber::set_global_default(subscriber).unwrap();*/
        
    
    let my_layer = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_target(true)
        .with_level(true)
        .compact()
        //.with_filter(my_filter)
        ;

    tracing_subscriber::registry()
        .with(my_layer)
        .init();



    // Initialize logging.
    /*let _ = env_logger::Builder::new()
        .format_timestamp_millis()
        .filter_level(cli.verbosity.log_level_filter())
        .build();*/
}
