use std::cell::RefCell;

use diesel::{SqliteConnection, Connection, QueryDsl, RunQueryDsl, ExpressionMethods, OptionalExtension};
use log::*;
use color_eyre::Result;

use crate::{
    stacks, 
    db::schema,
    db::model,
    context::{TestEnvPaths, boot_data::mainnet_boot_data, Network, BlockCursor, blocks::BlockHeader}
};

use super::ReadableEnv;

/// This environment type is read-only and reads directly from a Stacks node's
/// file/data structure. This can either be directly from a local node, or from
/// a data archive such as from the Hiro archive:
/// - mainnet: https://archive.hiro.so/mainnet/stacks-blockchain/
/// - testnet: https://archive.hiro.so/testnet/stacks-blockchain/
pub struct StacksNodeEnv<'a> {
    name: &'a str,
    node_dir: &'a str,
    paths: TestEnvPaths,
    index_db_conn: RefCell<SqliteConnection>,
    chainstate: stacks::StacksChainState,
    clarity_db_conn: SqliteConnection,
    sortition_db: stacks::SortitionDB,
}

impl<'a> StacksNodeEnv<'a> {
    /// Creates a new [StacksNodeEnv] instance from the specified node directory.
    /// The node directory should be working directory of the node, i.e.
    /// `/stacks-node/mainnet/` or `/stacks-node/testnet`.
    pub fn new(name: &'a str, node_dir: &'a str) -> Result<Self> {
        // Determine our paths.
        let paths = TestEnvPaths::new(node_dir);
        paths.print(name);

        debug!("[{name}] loading index db...");
        let mut index_db_conn = SqliteConnection::establish(&paths.index_db_path)?;
        info!("[{name}] successfully connected to index db");

        // Stacks nodes contain a db configuration in their index database's
        // `db_config` table which indicates version, network and chain id. Retrieve
        // this information and use it for setting up our readers.
        let db_config = schema::chainstate_marf::db_config::table
            .first::<model::chainstate_db::DbConfig>(&mut index_db_conn)?;

        // Convert the db config to a Network variant incl. chain id.
        let network = if db_config.mainnet {
            Network::Mainnet(db_config.chain_id as u32)
        } else {
            Network::Testnet(db_config.chain_id as u32)
        };

        // Setup our options for the Marf.
        let mut marf_opts = stacks::MARFOpenOpts::default();
        marf_opts.external_blobs = true;

        debug!("opening chainstate");
        let (chainstate, _) = stacks::StacksChainState::open(
            network.is_mainnet(), 
            network.chain_id(), 
            &paths.chainstate_path, 
            Some(marf_opts))?;
        info!("[{name}] successfully opened chainstate");

        debug!("[{name}] loading clarity db...");
        let clarity_db_conn = SqliteConnection::establish(&paths.clarity_db_path)?;
        info!("[{name}] successfully connected to clarity db");

        //debug!("attempting to migrate sortition db");
        debug!("opening sortition db");
        let sortition_db = super::open_sortition_db(&paths.sortition_db_path, &network)?;
        info!("successfully opened sortition db");

        Ok(Self {
            name,
            node_dir,
            paths,
            index_db_conn: RefCell::new(index_db_conn),
            chainstate,
            clarity_db_conn,
            sortition_db
        })
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
}

impl ReadableEnv for StacksNodeEnv<'_> {
    /// Retrieve a cursor over all blocks.
    fn blocks(&self) -> Result<BlockCursor> {
        let headers = self.block_headers()?;
        let cursor = BlockCursor::new(&self.paths.blocks_dir, headers);
        Ok(cursor)
    }
}