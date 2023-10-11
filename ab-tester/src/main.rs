mod cli;
mod config;
mod context;
mod errors;
mod model;
mod schema;
#[macro_use]
mod macros;

use std::process::exit;

use anyhow::{bail, Result};
use blockstack_lib::chainstate::stacks::TransactionContractCall;
use clap::Parser;
use cli::*;
use context::TestContext;
use log::*;
use stacks_common::types::chainstate::StacksBlockId;

use crate::errors::AppError;

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("cli: {:?}", cli);

    // Initialize logging.
    env_logger::init();

    #[allow(clippy::single_match)]
    match cli.command {
        Commands::Tui(args) => {
            cmd_tui(args)?;
        }
        Commands::Data(args) => {
            cmd_data(args)?;
        }
    }

    ok!()
}

fn cmd_tui(tui_args: TuiArgs) -> Result<()> {
    let _ = tui_args;
    todo!()
}

fn cmd_data(data_args: DataArgs) -> Result<()> {
    let config = config::Config::load()?;

    let mut test_context = TestContext::new(&config.chainstate.path)?;

    test_context
        .with_baseline_env(|_ctx, env| {
            let mut contract_calls: Vec<TransactionContractCall> = Default::default();

            info!(
                "aggregating contract calls starting at block height {}...",
                data_args.from_height
            );
            let mut processed_block_count = 0;
            for block_header in env.into_iter() {
                // Ensure that we've reached the specified block-height before beginning
                // processing.
                if block_header.block_height() < data_args.from_height {
                    continue;
                }

                info!("processing block #{}", block_header.block_height());

                data_args.assert_max_processed_block_count(processed_block_count)?;

                data_args.assert_block_height_under_max_height(block_header.block_height())?;

                if block_header.block_height() % 1000 == 0 {
                    info!(
                        "... at block #{}, contract call count: {}",
                        block_header.block_height(),
                        contract_calls.len()
                    );
                }

                /*info!(
                    "processing block: {{ height = {}, hash = '{}' }}",
                    block_header.block_height, block_header.index_block_hash,
                );*/

                if block_header.is_genesis() {
                    debug!(
                        "genesis block - skipping '{}'",
                        block_header.index_block_hash
                    );
                    continue;
                }

                let block_id = StacksBlockId::from_hex(&block_header.index_block_hash)?;
                let block = env.get_stacks_block(&block_header.index_block_hash)?;

                env.load_block(&block_id)?;

                for tx in block.txs {
                    use blockstack_lib::chainstate::stacks::TransactionPayload::*;

                    match tx.payload {
                        ContractCall(contract_call) => {
                            let _contract_id = &contract_call.contract_identifier();
                            contract_calls.push(contract_call);

                            //trace!("contract call {{ contract id: '{}' }}", contract_id);
                            //env.load_contract_analysis(&block_id, contract_id)?;
                            //trace!("{:?}", contract);
                            //panic!("exit here")
                        }
                        SmartContract(_contract, clarity_version) => {
                            //info!("{{ block_id: {}, index_block_hash: {}, block_hash: {} }}", block_info.0, block_info.4, block_info.1);
                            //info!("contract: {:?}; clarity_version: {:?}", contract, clarity_version);
                        }
                        _ => {}
                    }
                }

                processed_block_count += 1;
            }
            info!(
                "finished aggregating {} contract calls.",
                contract_calls.len()
            );

            ok!()
        })
        .map_err(|err| match err.downcast_ref() {
            Some(AppError::Graceful(graceful)) => {
                info!("terminating gracefully: {graceful:?}");
                exit(0)
            }
            _ => {
                error!("encountered a fatal error: {err:?}");
                exit(2)
            }
        })
}
