mod config;
mod context;
mod model;
mod schema;

use anyhow::Result;
use blockstack_lib::chainstate::stacks::index::marf::MARFOpenOpts;
use blockstack_lib::clarity_vm::database::marf::MarfedKV;
use context::TestContext;
use log::*;
use stacks_common::types::chainstate::StacksBlockId;


fn main() -> Result<()> {
    // Initialize logging.
    env_logger::init();

    let config = config::Config::load()?;

    let mut test_context = TestContext::new(&config.chainstate.path)?;

    test_context.with_baseline_env(|_ctx, env| {
        let iter = env.into_iter();
        debug!("iter: {:?}", iter);
        for block_info in env.fetch_staging_blocks()?.iter() {
            let _block_id = StacksBlockId::from_hex(&block_info.index_block_hash)?;
            let block = env.get_stacks_block(&block_info.index_block_hash)?;

            info!(
                "processing: {{ block height = {}, block hash = '{}' }}",
                block_info.block_height, block_info.index_block_hash
            );

            for tx in block.txs {
                use blockstack_lib::chainstate::stacks::TransactionPayload::*;
                match tx.payload {
                    ContractCall(contract_call) => {
                        let contract_id = contract_call.contract_identifier();
                        trace!("{:?}", contract_call);
                    }
                    SmartContract(contract, clarity_version) => {
                        //info!("{{ block_id: {}, index_block_hash: {}, block_hash: {} }}", block_info.0, block_info.4, block_info.1);
                        //info!("contract: {:?}; clarity_version: {:?}", contract, clarity_version);
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    })?;

    Ok(())
}



fn get_clarity_marfed_kv(path: &str) -> Result<MarfedKV> {
    let mut marf_open_opts = MARFOpenOpts::default();
    marf_open_opts.external_blobs = true;

    let marfed_kv = MarfedKV::open(path, None, Some(marf_open_opts))?;
    Ok(marfed_kv)
}
