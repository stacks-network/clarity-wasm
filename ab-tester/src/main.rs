mod schema;
mod model;

use blockstack_lib::chainstate::stacks::{db::StacksChainState, StacksBlock};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use schema::*;
use log::*;
use stacks_common::types::chainstate::{ConsensusHash, BlockHeaderHash, StacksBlockId};

fn main() {
    // Initialize logging.
    env_logger::init();

    // Connect to the database.
    let db_path = "/media/cylwit/data/stacks-blockchain/mainnet-stacks-blockchain-latest/mainnet/chainstate/vm/index.sqlite";
    let db = &mut SqliteConnection::establish(db_path)
        .unwrap_or_else(|e| panic!("Error connecting to database: {:?}", e));

    info!("Successfully connected to database at {}", db_path);

    let tmp = block_headers::table
        .inner_join(
            marf_data::table.on(marf_data::block_hash.eq(block_headers::index_block_hash))
        )
        .order_by(marf_data::block_id.asc())
        .offset(1) // Don't load genesis block
        .limit(1000)
        .select((
            marf_data::block_id,
            marf_data::block_hash,
            block_headers::consensus_hash,
            block_headers::parent_block_id,
            block_headers::index_block_hash
        ));

    let blocks_dir = "/media/cylwit/data/stacks-blockchain/mainnet-stacks-blockchain-latest/mainnet/chainstate/blocks/";

    debug!("Retrieving block information");
    let results = tmp.get_results::<(i32, String, String, String, String)>(db)
        .expect("Failed to retrieve result");

    for block_info in results.iter() {

        info!("{{ block_id: {}, index_block_hash: {}, block_hash: {} }}", block_info.0, block_info.4, block_info.1);
        let index_block_hash = StacksBlockId::from_hex(&block_info.4)
            .unwrap_or_else(|_| { panic!("Failed to parse stacks block id for block #{:?}", &block_info.0) });

        //debug!("Getting path for block");
        let index_block_path = StacksChainState::get_index_block_path(blocks_dir, &index_block_hash)
            .unwrap_or_else(|_| { panic!("Failed to get index path for block #{:?}", &block_info.0) });

        //debug!("Reading block");
        let block: StacksBlock = StacksChainState::consensus_load(&index_block_path)
            .unwrap_or_else(|_| panic!("Failed to load block."));
        
        //info!("block: {:?}", block);
        for tx in block.txs {
            use blockstack_lib::chainstate::stacks::TransactionPayload::*;
            match tx.payload {
                ContractCall(contract_call) => {
                    info!("{:?}", contract_call);
                },
                SmartContract(contract, clarity_version) => {
                    info!("contract: {:?}; clarity_version: {:?}", contract, clarity_version);
                },
                _ => {}
            }
        }
    }
}

