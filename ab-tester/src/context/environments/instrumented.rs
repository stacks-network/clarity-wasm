use std::cell::RefCell;

use diesel::{SqliteConnection, Connection};
use log::*;
use color_eyre::Result;

use crate::{context::{Runtime, Network, TestEnvPaths, boot_data::mainnet_boot_data}, stacks};

/// This environment type is app-specific and will instrument all Clarity-related
/// operations. This environment can be used for comparisons.
pub struct InstrumentedEnv<'a> {
    working_dir: &'a str,
    runtime: Runtime,
    network: Network,
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    sortition_db: stacks::SortitionDB
}

impl<'a> InstrumentedEnv<'a> {
    /// Creates a new [InstrumentedEnv]. This method expects the provided
    /// `working_dir` to either be uninitialized or be using the same [Runtime]
    /// and [Network] configuration.
    pub fn new(name: &'a str, working_dir: &'a str, runtime: Runtime, network: Network) -> Result<Self> {
        let paths = TestEnvPaths::new(working_dir);
        paths.print(name);

        // Setup our options for the Marf.
        let mut marf_opts = stacks::MARFOpenOpts::default();
        marf_opts.external_blobs = true;

        // Setup our boot data to be used if the chainstate hasn't been initialized yet.
        let mut boot_data = if network.is_mainnet() {
            mainnet_boot_data()
        } else {
            todo!("testnet not yet supported")
        };

        debug!("initializing chainstate");
        let (chainstate, _) = stacks::StacksChainState::open_and_exec(
            network.is_mainnet(),
            1,
            &paths.chainstate_path,
            Some(&mut boot_data),
            Some(marf_opts.clone()),
        )?;
        info!("[{name}] chainstate initialized.");

        debug!("[{name}] loading index db...");
        let index_db_conn = SqliteConnection::establish(&paths.index_db_path)?;
        info!("[{name}] successfully connected to index db");

        debug!("[{name}] loading clarity db...");
        let clarity_db_conn = SqliteConnection::establish(&paths.clarity_db_path)?;
        info!("[{name}] successfully connected to clarity db");

        //debug!("attempting to migrate sortition db");
        debug!("opening sortition db");
        let sortition_db = super::open_sortition_db(&paths.sortition_db_path, &network)?;
        info!("successfully opened sortition db");

        Ok(Self {
            working_dir,
            runtime,
            network,
            chainstate,
            index_db_conn: RefCell::new(index_db_conn),
            clarity_db_conn,
            sortition_db
        })
    }
}