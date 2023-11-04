use std::cell::RefCell;

use blockstack_lib::chainstate::stacks::index::{self, ClarityMarfTrieId, MarfTrieId};
use color_eyre::{eyre::bail, Result};
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use crate::{
    appdb::AppDb,
    clarity,
    clarity::ClarityConnection,
    datastore::DataStore,
    model, ok,
    schema::{self, chainstate_marf},
    stacks,
};

use super::{
    blocks::{BlockCursor, BlockHeader},
    Block,
};

pub enum Runtime {
    Interpreter = 1,
    Wasm = 2,
}

/// Holds all state between environments as well as the
pub struct GlobalEnvContext {
    app_db: AppDb,
}

impl GlobalEnvContext {
    pub fn new(app_db: AppDb) -> Self {
        Self { app_db }
    }

    /// Get or create a a test environment.
    pub fn env(&self, name: &str, runtime: Runtime, stacks_dir: &str) -> Result<TestEnv> {
        let env_id = if let Some(db_env) = self.app_db.get_env(name)? {
            db_env.id
        } else {
            let db_env = self.app_db.new_env(name, runtime as i32)?;
            db_env.id
        };

        TestEnv::new(env_id, name, stacks_dir, self)
    }
}

/// Container for a test environment.
pub struct TestEnv<'a> {
    id: i32,
    name: String,
    ctx: &'a GlobalEnvContext,
    //chainstate_path: String,
    blocks_dir: String,
    chainstate: stacks::StacksChainState,
    index_db_conn: RefCell<SqliteConnection>,
    sortition_db: stacks::SortitionDB,
    clarity_db_conn: SqliteConnection,
    burnchain: stacks::Burnchain,
}

impl std::fmt::Debug for TestEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestEnv")
            //.field("chainstate_path", &self.chainstate_path)
            .field("blocks_dir", &self.blocks_dir)
            .field("chainstate", &"...")
            .field("index_db", &"...")
            .field("sortition_db", &"...")
            .field("clarity_db", &"...")
            .finish()
    }
}

impl<'a> TestEnv<'a> {
    /// Creates a new instance of a [TestEnv] and attempts to open its database files.
    pub fn new(id: i32, name: &str, stacks_dir: &str, ctx: &'a GlobalEnvContext) -> Result<Self> {
        let index_db_path = format!("{}/chainstate/vm/index.sqlite", stacks_dir);
        let sortition_db_path = format!("{}/burnchain/sortition", stacks_dir);
        let blocks_dir = format!("{}/chainstate/blocks", stacks_dir);
        let chainstate_path = format!("{}/chainstate", stacks_dir);
        let clarity_db_path = format!("{}/chainstate/vm/clarity/marf.sqlite", stacks_dir);

        debug!("[{name}] index_db_path: '{}'", index_db_path);
        debug!("sortition_db_path: '{}'", sortition_db_path);
        debug!("[{name}] blocks_dir: '{}'", blocks_dir);

        let mut boot_data = stacks::ChainStateBootData {
            initial_balances: vec![],
            post_flight_callback: None,
            first_burnchain_block_hash: stacks::BurnchainHeaderHash::from_hex(
                stacks::BITCOIN_MAINNET_FIRST_BLOCK_HASH,
            )
            .unwrap(),
            first_burnchain_block_height: stacks::BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT as u32,
            first_burnchain_block_timestamp: stacks::BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP,
            pox_constants: stacks::PoxConstants::mainnet_default(),
            get_bulk_initial_lockups: Some(Box::new(|| {
                Box::new(stacks::GenesisData::new(false).read_lockups().map(|item| {
                    stacks::ChainstateAccountLockup {
                        address: item.address,
                        amount: item.amount,
                        block_height: item.block_height,
                    }
                }))
            })),
            get_bulk_initial_balances: Some(Box::new(|| {
                Box::new(stacks::GenesisData::new(false).read_balances().map(|item| {
                    stacks::ChainstateAccountBalance {
                        address: item.address,
                        amount: item.amount,
                    }
                }))
            })),
            get_bulk_initial_namespaces: Some(Box::new(|| {
                Box::new(
                    stacks::GenesisData::new(false)
                        .read_namespaces()
                        .map(|item| stacks::ChainstateBNSNamespace {
                            namespace_id: item.namespace_id,
                            importer: item.importer,
                            buckets: item.buckets,
                            base: item.base as u64,
                            coeff: item.coeff as u64,
                            nonalpha_discount: item.nonalpha_discount as u64,
                            no_vowel_discount: item.no_vowel_discount as u64,
                            lifetime: item.lifetime as u64,
                        }),
                )
            })),
            get_bulk_initial_names: Some(Box::new(|| {
                Box::new(stacks::GenesisData::new(false).read_names().map(|item| {
                    stacks::ChainstateBNSName {
                        fully_qualified_name: item.fully_qualified_name,
                        owner: item.owner,
                        zonefile_hash: item.zonefile_hash,
                    }
                }))
            })),
        };

        let mut marf_opts = stacks::MARFOpenOpts::default();
        marf_opts.external_blobs = true;

        debug!("initializing chainstate");
        let (chainstate, boot_receipt) = stacks::StacksChainState::open_and_exec(
            true,
            1,
            &format!("{}/chainstate", stacks_dir),
            Some(&mut boot_data),
            Some(marf_opts.clone()),
        )?;
        info!("[{name}] chainstate initialized.");

        debug!("[{name}] loading index db...");
        let index_db_conn = SqliteConnection::establish(&index_db_path)?;
        info!("[{name}] successfully connected to index db");

        debug!("[{name}] loading clarity db...");
        let clarity_db_conn = SqliteConnection::establish(&clarity_db_path)?;
        info!("[{name}] successfully connected to clarity db");

        /*debug!("[{name}] opening chainstate...");
        let chainstate =
            stacks::StacksChainState::open(true, 1, &chainstate_path, Some(marf_opts.clone()))?;
        info!("[{name}] successfully opened chainstate");*/

        debug!("[{name}] creating burnchain");
        let burnchain = stacks::Burnchain::new(stacks_dir, "bitcoin", "mainnet")?;

        //debug!("attempting to migrate sortition db");
        //stacks::SortitionDB::migrate_if_exists(&sortition_db_path, stacks::STACKS_EPOCHS_MAINNET.as_ref())?;
        debug!("migration successful; opening sortition db");
        let sortition_db = stacks::SortitionDB::connect(
            &sortition_db_path,
            stacks::BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
            &stacks::BurnchainHeaderHash::from_hex(stacks::BITCOIN_MAINNET_FIRST_BLOCK_HASH)
                .unwrap(),
            stacks::BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP.into(),
            stacks::STACKS_EPOCHS_MAINNET.as_ref(),
            stacks::PoxConstants::mainnet_default(),
            true,
        )?;
        info!("successfully opened sortition db");

        Ok(Self {
            id,
            name: name.to_string(),
            ctx,
            //chainstate_path: chainstate_path.to_string(),
            blocks_dir,
            chainstate,
            index_db_conn: RefCell::new(index_db_conn),
            sortition_db,
            clarity_db_conn,
            burnchain,
        })
    }

    /// Execute the given block with access to the underlying [ClarityDatabase].
    pub fn with_clarity_db(
        &self,
        f: impl FnOnce(&Self, &mut clarity::ClarityDatabase) -> Result<()>,
    ) -> Result<()> {
        let burndb = self.sortition_db.index_conn();
        let mut data_store = DataStore::new(&self.ctx.app_db);
        let rollback_wrapper = clarity::RollbackWrapper::new(&mut data_store);
        let mut clarity_db = clarity::ClarityDatabase::new_with_rollback_wrapper(
            rollback_wrapper,
            &clarity::NULL_HEADER_DB,
            &burndb,
        );

        clarity_db.begin();
        clarity_db.set_clarity_epoch_version(stacks::StacksEpochId::latest());
        clarity_db.commit();

        f(self, &mut clarity_db)
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

        let tip_parent = schema::chainstate_marf::block_headers::table
            .filter(chainstate_marf::block_headers::index_block_hash.eq(&tip.parent_block_id))
            .get_result::<model::chainstate_db::BlockHeader>(
                &mut *self.index_db_conn.borrow_mut(),
            )?;

        let mut current_block = Some(tip.clone());
        let mut headers = vec![BlockHeader::new(
            tip.block_height as u32,
            hex::decode(tip.index_block_hash)?,
            hex::decode(&tip.parent_block_id)?,
            hex::decode(tip.consensus_hash)?,
            hex::decode(tip_parent.consensus_hash)?,
        )];

        // Walk backwards
        while let Some(block) = current_block {
            let block_parent = schema::chainstate_marf::block_headers::table
                .filter(
                    schema::chainstate_marf::block_headers::index_block_hash
                        .eq(block.parent_block_id),
                )
                .get_result::<model::chainstate_db::BlockHeader>(
                    &mut *self.index_db_conn.borrow_mut(),
                )
                .optional()?;

            if let Some(b) = block_parent.clone() {
                let parent_consensus_hash = &block.consensus_hash;

                headers.push(BlockHeader::new(
                    b.block_height as u32,
                    hex::decode(b.index_block_hash)?,
                    hex::decode(&tip.parent_block_id)?,
                    hex::decode(&b.consensus_hash)?,
                    hex::decode(parent_consensus_hash)?,
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
        let cursor = BlockCursor::new(&self.blocks_dir, headers);
        Ok(cursor)
    }

    pub fn insert_block(&self, height: i32, block_hash: &[u8], index_hash: &[u8]) -> Result<()> {
        self.ctx
            .app_db
            .insert_block(self.id, height, block_hash, index_hash)?;

        ok!()
    }

    pub fn block_begin(&mut self, block: &Block) -> Result<()> {
        self.ctx.app_db.insert_block(
            self.id, 
            block.block_height()? as i32, 
            block.block_hash()?.as_bytes(), 
            block.index_block_hash()?
        )?;

        ok!()
    }

    pub fn test(&mut self, block: &Block, tx: &stacks::StacksTransaction) -> Result<()> {
        //let current_block_id: stacks::StacksBlockId;
        // TODO: Only if genesis
        let current_block_id = <stacks::StacksBlockId as ClarityMarfTrieId>::sentinel();
        let next_block_id: stacks::StacksBlockId;

        match block {
            Block::Boot(_) => bail!("cannot process the boot block"),
            Block::Genesis(_) => bail!("cannot process the genesis block"),
            Block::Regular(header, stacks_block) => {
                //current_block_id = header.parent_stacks_block_id()?;
                next_block_id = header.stacks_block_id()?;
            }
        };

        let burndb = self.sortition_db.index_conn();

        debug!("creating chainstate tx");
        let (chainstate_tx, clarity_instance) = self.chainstate.chainstate_tx_begin()?;
        debug!("chainstate tx started");

        debug!("beginning clarity block");
        let mut clarity_block_conn = clarity_instance.begin_block(
            &current_block_id,
            &next_block_id,
            &chainstate_tx,
            &burndb,
        );

        debug!("starting clarity tx processing");
        let clarity_tx_conn = clarity_block_conn.start_transaction_processing();

        debug!("returning");

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

    /// Loads the specified block from the MARF.
    pub fn load_block(&mut self, block_id: &stacks::StacksBlockId) -> Result<()> {
        debug!("beginning to walk the block: {}", block_id);
        let leaves = self.walk_block(block_id, false)?;

        if !leaves.is_empty() {
            debug!("finished walking, leaf count: {}", leaves.len());
        } else {
            warn!("no leaves found");
        }

        for leaf in leaves {
            let value = schema::clarity_marf::data_table::table
                .filter(schema::clarity_marf::data_table::key.eq(leaf.data.to_string()))
                .first::<model::clarity_db::DataEntry>(&mut self.clarity_db_conn)
                .optional()?;

            if let Some(value_unwrapped) = value {
                let clarity_value =
                    clarity::Value::try_deserialize_hex_untyped(&value_unwrapped.value);
                if let Ok(clarity_value) = clarity_value {
                    trace!("deserialized value: {:?}", &clarity_value);
                } else {
                    debug!("failed to deserialize value: {:?}", &value_unwrapped.value);
                }
            }
        }

        Ok(())
    }

    /// Helper function for [`Self::load_block()`] which is used to walk the MARF,
    /// looking for leaf nodes.
    ///
    /// If `follow_backptrs` is true, the entire MARF from genesis _up to and
    /// including the specified `block_id`_ will be read. At higher blocks heights this
    /// is very slow.
    fn walk_block(
        &mut self,
        block_id: &stacks::StacksBlockId,
        follow_backptrs: bool,
    ) -> Result<Vec<stacks::TrieLeaf>> {
        use stacks::*;

        let mut leaves: Vec<TrieLeaf> = Default::default();

        self.chainstate.with_clarity_marf(|marf| -> Result<()> {
            let mut marf = marf.reopen_readonly()?;
            let _root_hash = marf.get_root_hash_at(block_id)?;

            let _ = marf.with_conn(|storage| -> Result<()> {
                debug!("opening block {block_id}");
                storage.open_block(block_id)?;
                let (root_node_type, _) = Trie::read_root(storage)?;

                let mut level: u32 = 0;
                Self::inner_walk_block(
                    storage,
                    &root_node_type,
                    &mut level,
                    follow_backptrs,
                    &mut leaves,
                )?;

                Ok(())
            });
            Ok(())
        })?;

        Ok(leaves)
    }

    /// Helper function for [`Self::walk_block()`] which is used for recursion
    /// through the [MARF](blockstack_lib::chainstate::stacks::index::MARF).
    fn inner_walk_block<T: stacks::MarfTrieId>(
        storage: &mut stacks::TrieStorageConnection<T>,
        node: &stacks::TrieNodeType,
        level: &mut u32,
        follow_backptrs: bool,
        leaves: &mut Vec<stacks::TrieLeaf>,
    ) -> Result<()> {
        use stacks::*;

        *level += 1;
        let node_type_id = TrieNodeID::from_u8(node.id()).unwrap();
        debug!(
            "[level {level}] processing {node_type_id:?} with {} ptrs",
            &node.ptrs().len()
        );

        match &node {
            TrieNodeType::Leaf(leaf) => {
                leaves.push(leaf.clone());
                *level -= 1;
                trace!("[level {level}] returned to level");
                return Ok(());
            }
            _ => {
                let mut ptr_number = 0;
                for ptr in node.ptrs().iter() {
                    ptr_number += 1;
                    trace!("[level {level}] [ptr no. {ptr_number}] ptr: {ptr:?}");

                    if is_backptr(ptr.id) {
                        if !follow_backptrs {
                            continue;
                        }
                        // Handle back-pointers

                        // Snapshot the current block hash & id so that we can rollback
                        // to them after we're finished processing this back-pointer.
                        let (current_block, current_id) = storage.get_cur_block_and_id();

                        // Get the block hash for the block the back-pointer is pointing to
                        let back_block_hash =
                            storage.get_block_from_local_id(ptr.back_block())?.clone();

                        trace!("[level {level}] following backptr: {ptr:?}, {back_block_hash}");

                        // Open the block to which the back-pointer is pointing.
                        storage.open_block_known_id(&back_block_hash, ptr.back_block())?;

                        // Read the back-pointer type.
                        let backptr_node_type =
                            storage.read_nodetype_nohash(&ptr.from_backptr())?;

                        // Walk the newly opened block using the back-pointer.
                        Self::inner_walk_block(
                            storage,
                            &backptr_node_type,
                            level,
                            follow_backptrs,
                            leaves,
                        )?;

                        // Return to the previous block
                        trace!(
                            "[level {level}] returning to context: {current_block} {current_id:?}"
                        );
                        storage.open_block_known_id(&current_block, current_id.unwrap())?;
                    } else {
                        trace!("[level {level}] following normal ptr: {ptr:?}");
                        // Snapshot the current block hash & id so that we can rollback
                        // to them after we're finished processing this back-pointer.
                        let (current_block, current_id) = storage.get_cur_block_and_id();
                        trace!(
                            "[level {level}] current block: {} :: {current_block}",
                            current_id.unwrap()
                        );

                        // Handle nodes contained within this block/trie
                        trace!("hello");
                        let type_id = TrieNodeID::from_u8(ptr.id()).unwrap();
                        if type_id == TrieNodeID::Empty {
                            trace!("[level {level}] reached empty node, continuing");
                            continue;
                        }

                        trace!("[level {level}] ptr node type: {type_id:?}");
                        let node_type = storage.read_nodetype_nohash(ptr).unwrap();

                        trace!(
                            "[level {level}] {:?} => {ptr:?}, ptrs: {}",
                            TrieNodeID::from_u8(ptr.id()),
                            node_type.ptrs().len()
                        );
                        Self::inner_walk_block(
                            storage,
                            &node_type,
                            level,
                            follow_backptrs,
                            leaves,
                        )?;
                    }
                }
            }
        }

        *level -= 1;
        trace!("[level {level}] returned to level");
        Ok(())
    }

    /// Loads the block with the specified block hash from chainstate (the `blocks`
    /// directory for the node).
    pub fn get_stacks_block(&self, block_hash: &str) -> Result<stacks::StacksBlock> {
        let block_id = stacks::StacksBlockId::from_hex(block_hash)?;
        let block_path =
            stacks::StacksChainState::get_index_block_path(&self.blocks_dir, &block_id)?;
        let block = stacks::StacksChainState::consensus_load(&block_path)?;

        Ok(block)
    }
}
