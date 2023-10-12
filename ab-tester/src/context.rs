use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Cursor, Read},
    rc::Rc,
};

use anyhow::{bail, Context, Result};
use blockstack_lib::{
    burnchains::PoxConstants,
    chainstate::{
        burn::db::sortdb::SortitionDB,
        stacks::{
            db::StacksChainState,
            index::{
                marf::{MARFOpenOpts, MarfConnection},
                node::{is_backptr, TrieNodeID, TrieNodeType, TriePath},
                storage::TrieStorageConnection,
                trie::Trie,
                MarfTrieId, TrieLeaf,
            },
            StacksBlock,
        },
    },
    core::{
        BITCOIN_MAINNET_FIRST_BLOCK_HASH, BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
        BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP, STACKS_EPOCHS_MAINNET,
    },
};
use clarity::vm::{
    clarity::ClarityConnection,
    database::{NULL_BURN_STATE_DB, NULL_HEADER_DB},
    types::{QualifiedContractIdentifier, TypeSignature},
    Value, analysis::ContractAnalysis, ClarityName,
};
use diesel::{
    sql_query, Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    SqliteConnection,
};
use log::*;
use rand::Rng;
use stacks_common::types::{
    chainstate::{BurnchainHeaderHash, StacksBlockId},
    StacksEpochId,
};

use crate::{
    model::{
        chainstate_db::BlockHeader, 
        clarity_db::DataEntry
    },
    schema::clarity_marf::data_table, config::Config,
};

pub struct TestContext {
    id: u64,
    baseline_env: Rc<RefCell<TestEnv>>,
    test_envs: HashMap<String, Rc<RefCell<TestEnv>>>,
    appdb: SqliteConnection
}

impl std::fmt::Debug for TestContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestContext")
            .field("id", &self.id)
            .field("baseline_env", &self.baseline_env)
            .field("test_envs", &self.test_envs)
            .field("appdb", &"...")
            .finish()
    }
}

impl TestContext {
    pub fn new(config: &Config) -> Result<Self> {
        let baseline_env = TestEnv::new(&config.chainstate.path)?;

        let appdb = SqliteConnection::establish(&config.app.db_path)?;

        Ok(Self {
            id: rand::thread_rng().gen_range(1000000000..9999999999),
            baseline_env: Rc::new(RefCell::new(baseline_env)),
            test_envs: Default::default(),
            appdb
        })
    }

    pub fn with_baseline_env(
        &mut self,
        f: impl FnOnce(&TestContext, &TestEnvContext) -> Result<()>,
    ) -> Result<()> {
        let env_ctx = TestEnvContext::new(self, Rc::clone(&self.baseline_env));
        f(self, &env_ctx)?;
        Ok(())
    }

    pub fn new_env(&mut self, name: &str) -> Result<()> {
        let dir = format!("{}/{}/chainstate", std::env::temp_dir().display(), self.id);
        let env = Rc::new(RefCell::new(TestEnv::new(&dir)?));
        self.test_envs.insert(name.to_string(), env);
        Ok(())
    }

    pub fn with_env(
        &mut self,
        name: &str,
        f: impl FnOnce(&TestContext, Option<&mut TestEnvContext>) -> Result<()>,
    ) -> Result<()> {
        if let Some(env) = self.test_envs.get(name) {
            let env_ctx = TestEnvContext::new(self, Rc::clone(env));
            todo!()
        } else {
            f(self, None)?;
            Ok(())
        }
    }
}

pub struct TestEnv {
    chainstate_path: String,
    blocks_dir: String,
    chainstate: StacksChainState,
    index_db: SqliteConnection,
    sortition_db: SortitionDB,
    clarity_db: SqliteConnection,
}

impl std::fmt::Debug for TestEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestEnv")
            .field("chainstate_path", &self.chainstate_path)
            .field("blocks_dir", &self.blocks_dir)
            .field("chainstate", &"...")
            .field("index_db", &"...")
            .field("sortition_db", &"...")
            .field("clarity_db", &"...")
            .finish()
    }
}

impl TestEnv {
    pub fn new(stacks_dir: &str) -> Result<Self> {
        let index_db_path = format!("{}/chainstate/vm/index.sqlite", stacks_dir);
        let sortition_db_path = format!("{}/burnchain/sortition", stacks_dir);
        let blocks_dir = format!("{}/chainstate/blocks", stacks_dir);
        let chainstate_path = format!("{}/chainstate", stacks_dir);
        let clarity_db_path = format!("{}/chainstate/vm/clarity/marf.sqlite", stacks_dir);

        debug!("index_db_path: '{}'", index_db_path);
        debug!("sortition_db_path: '{}'", sortition_db_path);
        debug!("blocks_dir: '{}'", blocks_dir);

        debug!("loading index db...");
        let index_db = SqliteConnection::establish(&index_db_path)?;
        info!("successfully connected to index db");

        debug!("loading clarity db...");
        let clarity_db = SqliteConnection::establish(&clarity_db_path)?;
        info!("successfully connected to clarity db");

        let mut marf_opts = MARFOpenOpts::default();
        marf_opts.external_blobs = true;

        debug!("opening chainstate...");
        let chainstate = StacksChainState::open(true, 1, &chainstate_path, Some(marf_opts))?;
        info!("successfully opened chainstate");

        debug!("opening sortition db...");
        let sortition_db = SortitionDB::connect(
            &sortition_db_path,
            BITCOIN_MAINNET_FIRST_BLOCK_HEIGHT,
            &BurnchainHeaderHash::from_hex(BITCOIN_MAINNET_FIRST_BLOCK_HASH).unwrap(),
            BITCOIN_MAINNET_FIRST_BLOCK_TIMESTAMP.into(),
            STACKS_EPOCHS_MAINNET.as_ref(),
            PoxConstants::mainnet_default(),
            false,
        )?;
        info!("successfully opened sortition db");

        Ok(Self {
            chainstate_path: chainstate_path.to_string(),
            blocks_dir,
            chainstate: chainstate.0,
            index_db,
            sortition_db,
            clarity_db,
        })
    }
}

/// Represents a Clarity smart contract.
#[derive(Debug)]
pub struct Contract {
    analysis: ContractAnalysis,
}

impl Contract {
    pub fn new(analysis: ContractAnalysis) -> Self {
        Self {
            analysis
        }
    }

    pub fn contract_analysis(&self) -> &ContractAnalysis {
        &self.analysis
    }
}


#[derive(Debug)]
pub struct TestEnvContext<'a> {
    test_context: &'a TestContext,
    env: Rc<RefCell<TestEnv>>,
}

impl<'a> TestEnvContext<'a> {
    pub fn new(test_context: &'a TestContext, env: Rc<RefCell<TestEnv>>) -> Self {
        Self { test_context, env }
    }

    pub fn blocks(&self, start_at: u32) -> Result<BlockCursor> {
        let mut env = self.env.borrow_mut();

        let blocks_query = format!(
            "
            SELECT DISTINCT
                parent.block_height, 
                parent.index_block_hash, 
                parent.parent_block_id 
            FROM block_headers parent 
            INNER JOIN block_headers child ON child.parent_block_id = parent.index_block_hash
            WHERE parent.block_height >= {start_at}
            ORDER BY parent.block_height ASC;"
        );

        let headers = sql_query(blocks_query)
            .get_results::<BlockHeader>(&mut env.index_db)
            .context("Failed to retrieve block inventory.")?;

        debug!("retrieved {} block headers", headers.len());

        let cursor = BlockCursor::new(&env.blocks_dir, headers);
        Ok(cursor)
    }

    pub fn load_contract(
        &self,
        at_block: &StacksBlockId,
        contract_id: &QualifiedContractIdentifier,
    ) -> Result<()> {
        use clarity::vm::database::{ClarityDatabase, StoreType};

        let mut variable_paths: Vec<String> = Default::default();
        let mut env = self.env.borrow_mut();

        let mut conn = env.chainstate.clarity_state.read_only_connection(
            at_block,
            &NULL_HEADER_DB,
            &NULL_BURN_STATE_DB,
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
                let key =
                    ClarityDatabase::make_key_for_trip(contract_id, StoreType::Variable, name);

                let path = TriePath::from_key(&key);
                variable_paths.push(path.to_hex());
                //debug!("[{}](key='{}'; path='{}')", name, key, path);

                // Retrieve the current value.
                let value = clarity_db.lookup_variable(
                    contract_id,
                    name,
                    &meta,
                    &StacksEpochId::Epoch24,
                )?;

                trace!("[{}](key='{}'; path='{}'): {:?}", name, key, path, value);
            }

            // Handle maps
            for map in &contract_analysis.map_types {
                let meta = clarity_db.load_map(contract_id, map.0)?;
                //clarity_db.get_value("asdasdasdasdasdddsss", &TypeSignature::UIntType, &StacksEpochId::Epoch24)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Loads the specified block from the MARF.
    pub fn load_block(&self, block_id: &StacksBlockId) -> Result<()> {
        let mut env = self.env.borrow_mut();

        info!("beginning to walk the block: {}", block_id);
        let leaves = Self::walk_block(&mut env, block_id, false)?;

        if !leaves.is_empty() {
            info!("finished walking, leaf count: {}", leaves.len());
        } else {
            warn!("no leaves found");
        }

        for leaf in leaves {
            let value = data_table::table
                .filter(data_table::key.eq(leaf.data.to_string()))
                .first::<DataEntry>(&mut env.clarity_db)
                .optional()?;

            if let Some(value_unwrapped) = value {
                let clarity_value = Value::try_deserialize_hex_untyped(&value_unwrapped.value);
                if let Ok(clarity_value) = clarity_value {
                    trace!("deserialized value: {:?}", &clarity_value);
                } else {
                    warn!("failed to deserialize value: {:?}", &value_unwrapped.value);
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
    fn walk_block(env: &mut TestEnv, block_id: &StacksBlockId, follow_backptrs: bool) -> Result<Vec<TrieLeaf>> {
        let mut leaves: Vec<TrieLeaf> = Default::default();

        env.chainstate.with_clarity_marf(|marf| -> Result<()> {
            let mut marf = marf.reopen_readonly()?;
            let _root_hash = marf.get_root_hash_at(block_id)?;

            let _ = marf.with_conn(|storage| -> Result<()> {
                debug!("opening block {block_id}");
                storage.open_block(block_id)?;
                let (root_node_type, _) = Trie::read_root(storage)?;

                let mut level: u32 = 0;
                Self::inner_walk_block(storage, &root_node_type, &mut level, follow_backptrs, &mut leaves)?;

                Ok(())
            });
            Ok(())
        })?;

        Ok(leaves)
    }

    /// Helper function for [`Self::walk_block()`] which is used for recursion
    /// through the [MARF](blockstack_lib::chainstate::stacks::index::MARF).
    fn inner_walk_block<T: MarfTrieId>(
        storage: &mut TrieStorageConnection<T>,
        node: &TrieNodeType,
        level: &mut u32,
        follow_backptrs: bool,
        leaves: &mut Vec<TrieLeaf>,
    ) -> Result<()> {
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
                        Self::inner_walk_block(storage, &backptr_node_type, level, follow_backptrs, leaves)?;

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
                        Self::inner_walk_block(storage, &node_type, level, follow_backptrs, leaves)?;
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
    pub fn get_stacks_block(&self, block_hash: &str) -> Result<StacksBlock> {
        let env = self.env.borrow();

        let block_id = StacksBlockId::from_hex(block_hash)?;
        let block_path = StacksChainState::get_index_block_path(&env.blocks_dir, &block_id)?;
        let block = StacksChainState::consensus_load(&block_path)?;

        Ok(block)
    }
}

/// Provides a cursor for navigating and iterating through [Block]s.
pub struct BlockCursor {
    height: usize,
    blocks_dir: String,
    headers: Vec<BlockHeader>,
}

/// Manually implement [std::fmt::Debug] for [BlockCursor] since some fields
/// don't implement [std::fmt::Debug].
impl std::fmt::Debug for BlockCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockCursor")
            .field("pos", &self.height)
            .field("conn", &"...")
            .field("headers", &self.headers)
            .finish()
    }
}

/// Provides an implementation of [Iterator] for [BlockCursor] to allow for
/// simple iteration through blocks.
impl Iterator for BlockCursor {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        self.next().unwrap_or(None)
    }
}

/// Implementation of [BlockCursor].
#[allow(dead_code)]
impl BlockCursor {
    pub fn new(blocks_dir: &str, headers: Vec<BlockHeader>) -> Self {
        Self {
            blocks_dir: blocks_dir.to_string(),
            height: 0,
            headers,
        }
    }

    /// Gets the current position of the cursor.
    pub fn pos(&self) -> usize {
        self.height
    }

    /// Moves the cursor to the specified position.
    pub fn seek(&mut self, height: usize) -> Result<&mut Self> {
        if height >= self.headers.len() {
            bail!("Attempted to seek beyond chain tip");
        }
        self.height = height;
        Ok(self)
    }

    /// Retrieves the next [Block] relative the current block height and
    /// increments the cursor position.
    pub fn next(&mut self) -> Result<Option<Block>> {
        let height = self.height;

        if height >= self.headers.len() {
            return Ok(None);
        }

        self.height += 1;
        let block = self.get_block(height)?;
        Ok(block)
    }

    /// Decrements the cursor position and retrieves the [Block] at the
    /// decremented position (current position - 1). Returns [None] if there is
    /// no previous block (current block height is zero).
    pub fn prev(&mut self) -> Result<Option<Block>> {
        if self.height == 0 {
            return Ok(None);
        }

        self.height -= 1;
        let block = self.get_block(self.height)?;
        Ok(block)
    }

    /// Retrieves the [Block] at the specified height without moving the cursor.
    /// Returns [None] if there is no [Block] at the specified height.
    pub fn peek(&mut self, height: usize) -> Result<Option<Block>> {
        let block = self.get_block(height)?;
        Ok(block)
    }

    /// Retrieves the next [Block] without moving the cursor position. If the
    /// next height exceeds the chain tip this function will return [None].
    pub fn peek_next(&mut self) -> Result<Option<Block>> {
        let block = self.get_block(self.height + 1)?;
        Ok(block)
    }

    /// Retrieves the previous [Block] in relation to the current block height
    /// without moving the cursor position. If there is no previous block (the
    /// current height is zero) then this function will return [None].
    pub fn peek_prev(&mut self) -> Result<Option<Block>> {
        let block = self.get_block(self.height - 1)?;
        Ok(block)
    }

    /// Loads the block with the specified block hash from chainstate (the `blocks`
    /// directory for the node).
    fn get_block(&self, height: usize) -> Result<Option<Block>> {
        if height >= self.headers.len() {
            return Ok(None);
        }

        let header = self.headers[height].clone();

        // Get the block's path in chainstate.
        let block_id = StacksBlockId::from_hex(&header.index_block_hash)?;
        let block_path = StacksChainState::get_index_block_path(&self.blocks_dir, &block_id)?;
        // Load the block from chainstate.
        debug!("loading block with id {block_id} and path '{block_path}'");
        let stacks_block = StacksChainState::consensus_load(&block_path).ok();
        debug!("block loaded: {stacks_block:?}");

        let block = Block {
            header,
            block: stacks_block,
        };

        Ok(Some(block))
    }
}

/// Provides an abstracted view of a Stacks block as well as functions for
/// reading various information about blocks.
#[derive(Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub block: Option<StacksBlock>,
}

/// Implementation of [Block] which provides various functions to consumers for
/// reading information about a Stacks block.
#[allow(dead_code)]
impl Block {
    pub fn new(header: BlockHeader, block: StacksBlock) -> Self {
        Self {
            header,
            block: Some(block),
        }
    }

    pub fn block_height(&self) -> u32 {
        self.header.block_height()
    }

    pub fn is_genesis(&self) -> bool {
        self.header.is_genesis()
    }

    pub fn index_block_hash(&self) -> &str {
        &self.header.index_block_hash
    }
}
