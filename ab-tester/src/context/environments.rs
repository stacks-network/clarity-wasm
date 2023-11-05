use std::cell::RefCell;

use color_eyre::{eyre::bail, Result};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use crate::{
    clarity,
    clarity::ClarityConnection,
    context::boot_data::mainnet_boot_data,
    db::{appdb::AppDb, datastore::DataStore, model, schema},
    ok, stacks,
};

use self::{stacks_node::StacksNodeEnv, network::NetworkEnv, instrumented::InstrumentedEnv};

use super::{
    blocks::{BlockCursor, BlockHeader},
    Block, Network, Runtime, StoreType, TestEnvPaths,
};

mod stacks_node;
mod instrumented;
mod network;

/// Holds all state between environments.
pub struct GlobalEnvContext {
    app_db: AppDb,
}

impl GlobalEnvContext {
    pub fn new(app_db: AppDb) -> Self {
        Self { app_db }
    }

    /// Get or create a a test environment.
    pub fn env(
        &self,
        name: &str,
        runtime: Runtime,
        store_type: StoreType,
        network: Network,
        working_dir: &str,
    ) -> Result<TestEnv> {
        let env_id = if let Some(db_env) = self.app_db.get_env(name)? {
            db_env.id
        } else {
            let db_env = self.app_db.new_env(name, runtime as i32)?;
            db_env.id
        };

        TestEnv::new(env_id, name, working_dir, store_type, network, self)
    }
}

/// Contains a runtime environment configuration.
pub enum RuntimeEnv<'a> {
    StacksNode(StacksNodeEnv<'a>),
    Network(NetworkEnv<'a>),
    InstrumentedEnv(InstrumentedEnv<'a>)
}

impl<'a> RuntimeEnv<'a> {
    pub fn is_readonly(&self) -> bool {
        match self {
            Self::StacksNode(_) | Self::Network(_) => true,
            Self::InstrumentedEnv(_) => false
        }
    }

    pub fn from_stacks_node(name: &'a str, node_dir: &'a str) -> Result<Self> {
        Ok(
            Self::StacksNode(StacksNodeEnv::new(name, node_dir)?)
        )
    }
}

pub trait ReadableEnv {
    fn blocks(&self) -> Result<BlockCursor>;
}

pub trait WriteableEnv: ReadableEnv {
    
}

/// Container for a test environment.
pub struct TestEnv<'a> {
    id: i32,
    name: String,
    ctx: &'a GlobalEnvContext,
    store_type: StoreType,
    network: Network,
    //chainstate_path: String,
    paths: TestEnvPaths,
    chainstate: stacks::StacksChainState,
    index_db_conn: RefCell<SqliteConnection>,
    sortition_db: stacks::SortitionDB,
    clarity_db_conn: SqliteConnection,
    burnchain: stacks::Burnchain,

    block_tx_ctx: Option<BlockTransactionContext<'a, 'a>>,
}

impl std::fmt::Debug for TestEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestEnv")
            //.field("chainstate_path", &self.chainstate_path)
            .field("paths", &self.paths)
            .field("chainstate", &"...")
            .field("index_db", &"...")
            .field("sortition_db", &"...")
            .field("clarity_db", &"...")
            .finish()
    }
}

impl<'a> TestEnv<'a> {
    /// Creates a new instance of a [TestEnv] and attempts to open its database files.
    pub fn new(
        id: i32,
        name: &str,
        working_dir: &str,
        store_type: StoreType,
        network: Network,
        ctx: &'a GlobalEnvContext,
    ) -> Result<Self> {
        // Determine our paths.
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

        debug!("[{name}] creating burnchain");
        let burnchain = stacks::Burnchain::new(working_dir, "bitcoin", "mainnet")?;

        //debug!("attempting to migrate sortition db");
        debug!("opening sortition db");
        let sortition_db = Self::open_sortition_db(&paths.sortition_db_path, &network)?;
        info!("successfully opened sortition db");

        Ok(Self {
            id,
            name: name.to_string(),
            ctx,
            store_type,
            network,
            paths,
            chainstate,
            index_db_conn: RefCell::new(index_db_conn),
            sortition_db,
            clarity_db_conn,
            burnchain,

            block_tx_ctx: None,
        })
    }

    /// Opens the sortition DB baseed on the provided network.
    fn open_sortition_db(path: &str, network: &Network) -> Result<stacks::SortitionDB> {
        match network {
            Network::Mainnet(_) => {
                let boot_data = mainnet_boot_data();

                let sortition_db = stacks::SortitionDB::connect(
                    path,
                    stacks::BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
                    &stacks::BurnchainHeaderHash::from_hex(
                        stacks::BITCOIN_MAINNET_FIRST_BLOCK_HASH,
                    )
                    .unwrap(),
                    stacks::BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP.into(),
                    stacks::STACKS_EPOCHS_MAINNET.as_ref(),
                    boot_data.pox_constants,
                    true,
                )?;

                Ok(sortition_db)
            }
            Network::Testnet(_) => {
                todo!("testnet not yet supported")
            }
        }
    }

    /// Execute the given block with access to the underlying [ClarityDatabase].
    pub fn with_clarity_db(
        &self,
        f: impl FnOnce(&Self, &mut clarity::ClarityDatabase) -> Result<()>,
    ) -> Result<()> {
        let burn_state_db = self.sortition_db.index_conn();
        let mut backing_store = DataStore::new(&self.ctx.app_db);
        //let burn_state_db = DataStore::new(&self.ctx.app_db);

        let clarity_db = clarity::ClarityDatabase::new(
            &mut backing_store, 
            &self.ctx.app_db, 
            &burn_state_db);

        ok!()

        /*let rollback_wrapper = clarity::RollbackWrapper::new(&mut data_store);
        let mut clarity_db = clarity::ClarityDatabase::new_with_rollback_wrapper(
            rollback_wrapper,
            &clarity::NULL_HEADER_DB,
            &burndb,
        );

        clarity_db.begin();
        clarity_db.set_clarity_epoch_version(stacks::StacksEpochId::latest());
        clarity_db.commit();

        f(self, &mut clarity_db)*/
    }

    /// Retrieve all block headers from the underlying storage.
    fn block_headers(&self) -> Result<Vec<BlockHeader>> {
        // Retrieve the tip.
        let tip = schema::chainstate_marf::block_headers::table
            .order_by(schema::chainstate_marf::block_headers::block_height.desc())
            .limit(1)
            .get_result::<model::chainstate_db::BlockHeader>(
                &mut *self.index_db_conn.borrow_mut(),
            )?;

        let mut current_block = Some(tip);
        let mut headers: Vec<BlockHeader> = Vec::new();

        // Walk backwards
        while let Some(block) = current_block {
            let block_parent = schema::chainstate_marf::block_headers::table
                .filter(
                    schema::chainstate_marf::block_headers::index_block_hash
                        .eq(&block.parent_block_id),
                )
                .get_result::<model::chainstate_db::BlockHeader>(
                    &mut *self.index_db_conn.borrow_mut(),
                )
                .optional()?;

            if let Some(parent) = &block_parent {
                headers.push(BlockHeader::new(
                    block.block_height(),
                    hex::decode(block.index_block_hash)?,
                    hex::decode(block.parent_block_id)?,
                    hex::decode(block.consensus_hash)?,
                    hex::decode(&parent.consensus_hash)?,
                ));
            } else {
                headers.push(BlockHeader::new(
                    block.block_height(),
                    hex::decode(block.index_block_hash)?,
                    hex::decode(block.parent_block_id)?,
                    hex::decode(block.consensus_hash)?,
                    vec![0_u8; 20],
                ));
            }

            current_block = block_parent;
        }

        headers.reverse();
        debug!("first block: {:?}", headers[0]);
        debug!("tip: {:?}", headers[headers.len() - 1]);
        debug!("retrieved {} block headers", headers.len());

        Ok(headers)
    }

    /// Retrieve a cursor over all blocks.
    pub fn blocks(&self) -> Result<BlockCursor> {
        let headers = self.block_headers()?;
        let cursor = BlockCursor::new(&self.paths.blocks_dir, headers);
        Ok(cursor)
    }

    pub fn block_begin(
        &mut self,
        block: &Block,
        f: impl FnOnce(&mut BlockTransactionContext) -> Result<()>,
    ) -> Result<()> {
        let current_block_id: stacks::StacksBlockId;
        let next_block_id: stacks::StacksBlockId;

        match block {
            Block::Boot(_) => bail!("cannot process the boot block"),
            Block::Genesis(header) => {
                current_block_id = <stacks::StacksBlockId as stacks::ClarityMarfTrieId>::sentinel();
                next_block_id = header.stacks_block_id()?;
            }
            Block::Regular(header, _) => {
                current_block_id = header.parent_stacks_block_id()?;
                next_block_id = header.stacks_block_id()?;
            }
        };

        info!("current_block_id: {current_block_id}, next_block_id: {next_block_id}");

        // Insert this block into the app database.
        self.ctx.app_db.insert_block(
            self.id,
            block.block_height()? as i32,
            block.block_hash()?.as_bytes(),
            block.index_block_hash()?,
        )?;

        // We cannot process genesis as it was already processed as a part of chainstate
        // initialization. Log that we reached it and skip processing.
        if let Block::Genesis(_) = block {
            info!("genesis block cannot be processed as it was statically initialized; moving on");
            return ok!();
        }

        // Get an instance to the BurnStateDB (SortitionDB's `index_conn` implements this trait).
        let burn_db = self.sortition_db.index_conn();

        // Start a new chainstate transaction.
        debug!("creating chainstate tx");
        let (chainstate_tx, clarity_instance) = self.chainstate.chainstate_tx_begin()?;
        debug!("chainstate tx started");

        // Begin a new Clarity block.
        debug!("beginning clarity block");
        let mut clarity_block_conn = clarity_instance.begin_block(
            &current_block_id,
            &next_block_id,
            &chainstate_tx,
            &burn_db,
        );

        // Enter Clarity transaction processing for the new block.
        debug!("starting clarity tx processing");
        let clarity_tx_conn = clarity_block_conn.start_transaction_processing();

        // Call the provided function with our context object.
        //let mut block_tx_ctx = ;
        f(&mut BlockTransactionContext {
            clarity_tx_conn: &clarity_tx_conn,
        })?;

        debug!("returning");

        clarity_tx_conn.commit();
        clarity_block_conn.commit_to_block(&block.stacks_block_id()?);
        chainstate_tx.commit()?;

        ok!()
    }

    pub fn load_contract(
        &mut self,
        at_block: &stacks::StacksBlockId,
        contract_id: &clarity::QualifiedContractIdentifier,
    ) -> Result<()> {
        let mut variable_paths: Vec<String> = Default::default();

        let mut conn = self.chainstate.clarity_state.read_only_connection(
            at_block,
            &clarity::NULL_HEADER_DB,
            &clarity::NULL_BURN_STATE_DB,
        );

        conn.with_clarity_db_readonly(|clarity_db| {
            let contract_analysis = clarity_db.load_contract_analysis(contract_id);

            if contract_analysis.is_none() {
                bail!("Failed to load contract '{contract_id}'");
            }

            let contract_analysis = contract_analysis.unwrap();

            // Handle persisted variables.
            for (name, _) in contract_analysis.persisted_variable_types.iter() {
                // Get the metadata for the variable.
                let meta = clarity_db.load_variable(contract_id, name)?;

                // Construct the identifier (key) for this variable in the
                // persistence layer.
                let key = clarity::ClarityDatabase::make_key_for_trip(
                    contract_id,
                    clarity::StoreType::Variable,
                    name,
                );

                let path = stacks::TriePath::from_key(&key);
                variable_paths.push(path.to_hex());
                //debug!("[{}](key='{}'; path='{}')", name, key, path);

                // Retrieve the current value.
                let value = clarity_db.lookup_variable(
                    contract_id,
                    name,
                    &meta,
                    &stacks::StacksEpochId::Epoch24,
                )?;

                trace!("[{}](key='{}'; path='{}'): {:?}", name, key, path, value);
            }

            // Handle maps
            for map in &contract_analysis.map_types {
                let _meta = clarity_db.load_map(contract_id, map.0)?;
                //clarity_db.get_value("asdasdasdasdasdddsss", &TypeSignature::UIntType, &StacksEpochId::Epoch24)?;
            }

            Ok(())
        })?;

        Ok(())
    }
}

pub struct BlockTransactionContext<'a, 'b> {
    clarity_tx_conn: &'a stacks::ClarityTransactionConnection<'a, 'b>,
}


/// Opens the sortition DB baseed on the provided network.
fn open_sortition_db(path: &str, network: &Network) -> Result<stacks::SortitionDB> {
    match network {
        Network::Mainnet(_) => {
            let boot_data = mainnet_boot_data();

            let sortition_db = stacks::SortitionDB::connect(
                path,
                stacks::BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
                &stacks::BurnchainHeaderHash::from_hex(
                    stacks::BITCOIN_MAINNET_FIRST_BLOCK_HASH,
                )
                .unwrap(),
                stacks::BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP.into(),
                stacks::STACKS_EPOCHS_MAINNET.as_ref(),
                boot_data.pox_constants,
                true,
            )?;

            Ok(sortition_db)
        }
        Network::Testnet(_) => {
            todo!("testnet not yet supported")
        }
    }
}