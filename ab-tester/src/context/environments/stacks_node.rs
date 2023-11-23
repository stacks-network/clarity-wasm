use std::cell::RefCell;
use std::rc::Rc;

use color_eyre::eyre::{anyhow, bail};
use color_eyre::Result;
use diesel::{
    Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SqliteConnection,
};
use log::*;

use super::{BoxedDbIterResult, ReadableEnv, RuntimeEnv};
use crate::clarity::{self, ClarityConnection};
use crate::context::blocks::BlockHeader;
use crate::context::callbacks::{DefaultEnvCallbacks, RuntimeEnvCallbackHandler};
use crate::context::{BlockCursor, Network, StacksEnvPaths};
use crate::db::dbcursor::stream_results;
use crate::db::schema::sortition::*;
use crate::db::{model, schema};
use crate::{ok, stacks};

/// Holds initialization config for a [StacksNodeEnv].
pub struct StacksNodeEnvConfig {
    node_dir: String,
    paths: StacksEnvPaths,
}

/// Holds state for a [StacksNodeEnv].
pub struct StacksNodeEnvState {
    network: Network,
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    sortition_db_conn: Rc<RefCell<SqliteConnection>>,
    sortition_db: stacks::SortitionDB,
}

/// This environment type is read-only and reads directly from a Stacks node's
/// file/data structure. This can either be directly from a local node, or from
/// a data archive such as from the Hiro archive:
/// - mainnet: https://archive.hiro.so/mainnet/stacks-blockchain/
/// - testnet: https://archive.hiro.so/testnet/stacks-blockchain/
pub struct StacksNodeEnv {
    id: i32,
    name: String,
    env_config: StacksNodeEnvConfig,
    env_state: Option<StacksNodeEnvState>,
    callbacks: Box<dyn RuntimeEnvCallbackHandler>,
}

impl StacksNodeEnv {
    /// Creates a new [StacksNodeEnv] instance from the specified node directory.
    /// The node directory should be working directory of the node, i.e.
    /// `/stacks-node/mainnet/` or `/stacks-node/testnet`.
    pub fn new(id: i32, name: String, node_dir: String) -> Result<Self> {
        // Determine our paths.
        let paths = StacksEnvPaths::new(&node_dir);

        let env_config = StacksNodeEnvConfig { paths, node_dir };

        Ok(Self {
            id,
            name,
            env_config,
            env_state: None,
            callbacks: Box::<DefaultEnvCallbacks>::default(),
        })
    }

    /// Attempts to retrieve the [StacksNodeEnvState] for this environment. Will
    /// return an error if [RuntimeEnv::open] has not been called.
    fn get_env_state(&self) -> Result<&StacksNodeEnvState> {
        let state = self
            .env_state
            .as_ref()
            .ok_or(anyhow!("[{}] environment has not been opened", self.name))?;

        Ok(state)
    }

    /// Attempts to retrieve the [StacksNodeEnvState] for this environment as a
    /// mutable reference. Will return an error if [RuntimeEnv::open] has not been called.
    fn get_env_state_mut(&mut self) -> Result<&mut StacksNodeEnvState> {
        let state = self
            .env_state
            .as_mut()
            .ok_or(anyhow!("[{}] environment has not been opened", self.name))?;

        Ok(state)
    }

    /// Retrieve all block headers from the underlying storage.
    fn block_headers(&self) -> Result<Vec<BlockHeader>> {
        let name = &self.name;
        let state = self.get_env_state()?;

        // Retrieve the tip.
        self.callbacks.get_chain_tip_start(self);
        let tip = schema::chainstate_marf::block_headers::table
            .order_by(schema::chainstate_marf::block_headers::block_height.desc())
            .limit(1)
            .get_result::<model::chainstate_db::BlockHeader>(
                &mut *state.index_db_conn.borrow_mut(),
            )?;
        // TODO: Handle when there is no tip (chain uninitialized).
        self.callbacks
            .get_chain_tip_finish(self, tip.block_height as u32);
        let mut current_block = Some(tip);

        // Vec for holding the headers we run into. This will initially be
        // in reverse order (from tip to genesis) - we reverse it later.
        let mut headers: Vec<BlockHeader> = Vec::new();

        // Walk backwards from tip to genesis, following the canonical fork. We
        // do this so that we don't follow orphaned blocks/forks.
        self.callbacks.load_block_headers_start(self);
        while let Some(block) = current_block {
            let block_parent = schema::chainstate_marf::block_headers::table
                .filter(
                    schema::chainstate_marf::block_headers::index_block_hash
                        .eq(&block.parent_block_id),
                )
                .get_result::<model::chainstate_db::BlockHeader>(
                    &mut *state.index_db_conn.borrow_mut(),
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
            self.callbacks.load_block_headers_iter(self, headers.len());

            current_block = block_parent;
        }

        // Reverse the vec so that it is in block-ascending order.
        headers.reverse();

        debug!("[{name}] first block: {:?}", headers[0]);
        debug!("[{name}] tip: {:?}", headers[headers.len() - 1]);
        debug!("[{name}] retrieved {} block headers", headers.len());

        self.callbacks
            .load_block_headers_finish(self, headers.len());
        Ok(headers)
    }

    /// Loads a Clarity contract from chainstate for the provided
    /// [clarity::QualifiedContractIdentifier] at the given [stacks::StacksBlockId].
    fn load_contract(
        &mut self,
        at_block: &stacks::StacksBlockId,
        contract_id: &clarity::QualifiedContractIdentifier,
    ) -> Result<()> {
        let state = self.get_env_state_mut()?;
        let mut variable_paths: Vec<String> = Default::default();

        let mut conn = state.chainstate.clarity_state.read_only_connection(
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

    /// Retrieves all snapshots from the node's sortition database, ordered by
    /// block height ascending.
    fn sortition_snapshots(&self) -> Result<Vec<crate::types::Snapshot>> {
        let state = self.get_env_state()?;

        let results = snapshots::table
            .order_by(snapshots::block_height.asc())
            .get_results::<crate::db::model::sortition_db::Snapshot>(
                &mut *state.sortition_db_conn.borrow_mut(),
            )?
            .into_iter()
            .map(|s| {
                s.try_into()
                    .expect("failed to convert stacks node snapshot to common type")
            })
            .collect::<Vec<crate::types::Snapshot>>();

        Ok(results)
    }

    /// Retrieves all block commits from the node's sortition database, ordered by
    /// block height ascending.
    fn sortition_block_commits(&self) -> Result<Vec<crate::types::BlockCommit>> {
        let state = self.get_env_state()?;

        let results = block_commits::table
            .order_by(block_commits::block_height.asc())
            .get_results::<crate::db::model::sortition_db::BlockCommit>(
                &mut *state.sortition_db_conn.borrow_mut(),
            )?
            .into_iter()
            .map(|s| {
                s.try_into()
                    .expect("failed to convert stacks node block commit to common type")
            })
            .collect::<Vec<crate::types::BlockCommit>>();

        Ok(results)
    }

    /// Retrieves all epochs from the node's sortition database, ordered by
    /// epoch id ascending.
    fn sortition_epochs(&self) -> Result<Vec<crate::types::Epoch>> {
        let state = self.get_env_state()?;

        let results = epochs::table
            .order_by(epochs::epoch_id.asc())
            .get_results::<crate::db::model::sortition_db::Epoch>(
                &mut *state.sortition_db_conn.borrow_mut(),
            )?
            .into_iter()
            .map(|s| {
                s.try_into()
                    .expect("failed to convert stacks node epoch to common type")
            })
            .collect::<Vec<crate::types::Epoch>>();

        Ok(results)
    }

    /// Retrieves all ast rule heights from the node's sortition database, ordered by
    /// ast rule id ascending.
    fn sortition_ast_rule_heights(&self) -> Result<Vec<crate::types::AstRuleHeight>> {
        let state = self.get_env_state()?;

        let results = ast_rule_heights::table
            .order_by(ast_rule_heights::ast_rule_id.asc())
            .get_results::<crate::db::model::sortition_db::AstRuleHeight>(
                &mut *state.sortition_db_conn.borrow_mut(),
            )?
            .into_iter()
            .map(|s| {
                s.try_into()
                    .expect("failed to convert stacks ast rule height to common type")
            })
            .collect::<Vec<crate::types::AstRuleHeight>>();

        Ok(results)
    }
}

/// Implement [RuntimeEnv] for [StacksNodeEnv].
impl RuntimeEnv for StacksNodeEnv {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn is_readonly(&self) -> bool {
        true
    }

    fn is_open(&self) -> bool {
        self.env_state.is_some()
    }

    fn open(&mut self) -> Result<()> {
        let paths = &self.env_config.paths;
        let name = &self.name;

        self.callbacks.env_open_start(self, name);
        paths.print(name);

        debug!("[{name}] loading index db...");
        self.callbacks
            .open_index_db_start(self, &paths.index_db_path);
        let mut index_db_conn = SqliteConnection::establish(&paths.index_db_path)?;
        self.callbacks.open_index_db_finish(self);
        info!("[{name}] successfully connected to index db");

        // Stacks nodes contain a db configuration in their index database's
        // `db_config` table which indicates version, network and chain id. Retrieve
        // this information and use it for setting up our readers.
        self.callbacks.load_db_config_start(self);
        let db_config = schema::chainstate_marf::db_config::table
            .first::<model::chainstate_db::DbConfig>(&mut index_db_conn)?;
        self.callbacks.load_db_config_finish(self);

        // Convert the db config to a Network variant incl. chain id.
        self.callbacks.determine_network_start(self);
        let network = if db_config.mainnet {
            Network::Mainnet(db_config.chain_id as u32)
        } else {
            Network::Testnet(db_config.chain_id as u32)
        };
        self.callbacks.determine_network_finish(self, &network);

        // Setup our options for the Marf.
        let mut marf_opts = stacks::MARFOpenOpts::default();
        marf_opts.external_blobs = true;

        debug!("[{name}] opening chainstate");
        self.callbacks
            .open_chainstate_start(self, &paths.chainstate_dir);
        let (chainstate, _) = stacks::StacksChainState::open(
            network.is_mainnet(),
            network.chain_id(),
            &paths.chainstate_dir,
            Some(marf_opts),
        )?;
        self.callbacks.open_chainstate_finish(self);
        info!("[{name}] successfully opened chainstate");

        debug!("[{name}] loading clarity db...");
        self.callbacks
            .open_clarity_db_start(self, &paths.clarity_db_path);
        let clarity_db_conn = SqliteConnection::establish(&paths.clarity_db_path)?;
        self.callbacks.open_clarity_db_finish(self);
        info!("[{name}] successfully connected to clarity db");

        //debug!("attempting to migrate sortition db");
        debug!("[{name}] opening sortition db");
        self.callbacks
            .open_sortition_db_start(self, &paths.sortition_dir);
        let sortition_db_conn = SqliteConnection::establish(&paths.sortition_db_path)?;
        let sortition_db = super::open_sortition_db(&paths.sortition_dir, &network)?;
        self.callbacks.open_sortition_db_finish(self);
        info!("[{name}] successfully opened sortition db");

        let state = StacksNodeEnvState {
            network,
            index_db_conn: RefCell::new(index_db_conn),
            chainstate,
            clarity_db_conn,
            sortition_db_conn: Rc::new(RefCell::new(sortition_db_conn)),
            sortition_db,
        };

        self.env_state = Some(state);

        self.callbacks.env_open_finish(self);
        ok!()
    }
}

/// Implementation of [ReadableEnv] for [StacksNodeEnv].
impl ReadableEnv for StacksNodeEnv {
    /// Retrieve a cursor over all blocks.
    fn blocks(&self) -> Result<BlockCursor> {
        let headers = self.block_headers()?;
        let cursor = BlockCursor::new(&self.env_config.paths.blocks_dir, headers);
        Ok(cursor)
    }

    fn snapshots(&self) -> BoxedDbIterResult<crate::types::Snapshot> {
        let state = self.get_env_state()?;

        let result = stream_results::<
            crate::db::model::sortition_db::Snapshot,
            crate::types::Snapshot,
            _,
            _,
        >(
            snapshots::table.order_by(snapshots::block_height.asc()),
            state.sortition_db_conn.clone(),
            1000,
        );

        Ok(Box::new(result))
    }

    fn block_commits(&self) -> BoxedDbIterResult<crate::types::BlockCommit> {
        let state = self.get_env_state()?;

        let result = stream_results::<
            crate::db::model::sortition_db::BlockCommit,
            crate::types::BlockCommit,
            _,
            _,
        >(block_commits::table, state.sortition_db_conn.clone(), 1000);

        Ok(Box::new(result))
    }

    fn ast_rules(&self) -> BoxedDbIterResult<crate::types::AstRuleHeight> {
        todo!()
    }

    fn epochs(&self) -> BoxedDbIterResult<crate::types::Epoch> {
        todo!()
    }
}
