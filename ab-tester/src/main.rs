mod config;
mod context;
mod model;
mod schema;
mod cli;
#[macro_use]
mod macros;

use std::process::exit;

use anyhow::{Result, bail, Error};
use blockstack_lib::chainstate::stacks::TransactionContractCall;

use clap::Parser;
use context::TestContext;
use log::*;
use stacks_common::types::chainstate::StacksBlockId;
use cli::*;



fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("cli: {:?}", cli);

    // Initialize logging.
    env_logger::init();

    #[allow(clippy::single_match)]
    match cli.command {
        Commands::Tui(tui_args) => {
            
        },
        Commands::Data(data_args) => {
            cmd_data(data_args)?;
        }
    }

    ok!()
}

fn cmd_tui(tui_args: TuiArgs) -> Result<()> {
    ok!()
}

fn cmd_data(data_args: DataArgs) -> Result<()> {
    let config = config::Config::load()?;

    let mut test_context = TestContext::new(&config.chainstate.path)?;

    test_context.with_baseline_env(|_ctx, env| {
        let mut contract_calls: Vec<TransactionContractCall> = Default::default();
        
        info!("aggregating contract calls starting at block height {}...", data_args.from_height);
        let mut block_count = 0;
        for block_header in env.into_iter() {

            // Check if we've reached the specified max blocks processed count.
            if let Some(max_blocks) = data_args.max_block_count {
                if block_count >= max_blocks {
                    info!("reached max block count ({}), exiting.", max_blocks);
                    exit(0);
                }
            }

            if let Some(to_height) = data_args.to_height {
                if block_header.block_height > to_height - 1 {
                    info!("reached block height limit ({}), exiting.", block_header.block_height);
                    exit(0);
                }
            }
            

            if block_header.block_height < data_args.from_height {
                continue;
            }

            if block_header.block_height % 1000 == 0 {
                info!("... at block #{}, contract call count: {}", 
                    block_header.block_height,
                    contract_calls.len()
                );
            }
            /*info!(
                "processing block: {{ height = {}, hash = '{}' }}",
                block_header.block_height, block_header.index_block_hash,
            );*/

            if block_header.is_genesis() {
                debug!("genesis block - skipping '{}'", block_header.index_block_hash);
                continue;
            }

            let block_id = StacksBlockId::from_hex(&block_header.index_block_hash)?;
            let block = env.get_stacks_block(&block_header.index_block_hash)?;

            env.load_block(&block_id)?;

            for tx in block.txs {
                use blockstack_lib::chainstate::stacks::TransactionPayload::*;

                match tx.payload {
                    ContractCall(contract_call) => {
                        let contract_id = &contract_call.contract_identifier();
                        contract_calls.push(contract_call);
                        
                        //trace!("contract call {{ contract id: '{}' }}", contract_id);
                        //env.load_contract_analysis(&block_id, contract_id)?;
                        //trace!("{:?}", contract);
                        //panic!("exit here")
                    }
                    SmartContract(contract, clarity_version) => {
                        //info!("{{ block_id: {}, index_block_hash: {}, block_hash: {} }}", block_info.0, block_info.4, block_info.1);
                        //info!("contract: {:?}; clarity_version: {:?}", contract, clarity_version);
                    }
                    _ => {}
                }
            }

            block_count += 1;
        }
        info!("finished aggregating {} contract calls.", contract_calls.len());
        
        ok!()
    }).or_else(|e| {
        error!("Encountered error: {e:?}");
        bail!(e);
    })?;

    ok!()
}
