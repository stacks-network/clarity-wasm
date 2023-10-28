use crate::{cli::DataArgs, context::TestContext, ok, appdb::AppDb, runtime};
use clarity::vm::{types::{QualifiedContractIdentifier, StandardPrincipalData}, ClarityVersion};
use color_eyre::eyre::Result;
use blockstack_lib::chainstate::stacks::TransactionContractCall;
use diesel::{SqliteConnection, Connection};
use log::*;
use stacks_common::types::chainstate::StacksBlockId;

pub async fn exec(config: &crate::config::Config, data_args: DataArgs) -> Result<()> {

    let app_db_conn = SqliteConnection::establish(&config.app.db_path)?;
    let mut app_db = AppDb::new(app_db_conn);

    //crate::runtime::analyze_contract(contract_identifier, expressions, data_store, cost_tracker);
    //crate::runtime::install_contract(contract_identifier, expressions, clarity_db, cost_tracker);

    // Open a new context
    let mut test_context = TestContext::new(config)?;

    test_context.with_baseline_env(|_ctx, env| {
        let mut contract_calls: Vec<TransactionContractCall> = Default::default();

        info!(
            "aggregating contract calls starting at block height {}...",
            data_args.from_height
        );
        let mut processed_block_count = 0;
        for block_header in env.blocks(data_args.from_height)? {
            // Ensure that we've reached the specified block-height before beginning
            // processing.
            if block_header.block_height() < data_args.from_height {
                continue;
            }

            // Ensure that we haven't exceeded the specified max-blocks for processing.
            data_args.assert_max_processed_block_count(processed_block_count)?;

            // Ensure that we haven't reached the specified max block-height for processing.
            data_args.assert_block_height_under_max_height(block_header.block_height())?;

            //info!("processing block #{}", block_header.block_height());

            // We can't process the genesis block so skip it.
            if block_header.is_genesis() {
                debug!(
                    "genesis block - skipping '{}'",
                    block_header.index_block_hash()
                );
                continue;
            }

            let block_id = StacksBlockId::from_hex(block_header.index_block_hash())?;
            let block = env.get_stacks_block(block_header.index_block_hash())?;

            block.validate_transactions_static(mainnet, chain_id, epoch_id)

            // Load the block
            debug!("loading block '{block_id}'");
            env.load_block(&block_id)?;

            debug!("inserting block into app db");
            let db_block = app_db.insert_block(
                env.id(),
                block_header.header.block_height() as i32,
                block_header.header.block_height() as i32,
                block.block_hash().as_bytes(),
                &hex::decode(block_header.index_block_hash())?)?;

            for tx in block.txs {
                use blockstack_lib::chainstate::stacks::TransactionPayload::*;

                match &tx.payload {
                    ContractCall(contract_call) => {
                        let _contract_id = &contract_call.contract_identifier();
                        contract_calls.push(contract_call.clone());

                        //trace!("contract call {{ contract id: '{}' }}", contract_id);
                        //env.load_contract_analysis(&block_id, contract_id)?;
                        //trace!("{:?}", contract);
                        //panic!("exit here")
                    }
                    SmartContract(contract, clarity_version) => {
                        //info!("{{ block_id: {}, index_block_hash: {}, block_hash: {} }}", block_info.0, block_info.4, block_info.1);

                        let principal = StandardPrincipalData::from(tx.origin_address());
                        let contract_id = QualifiedContractIdentifier::new(principal, contract.name.clone());

                        info!("tx_id: {:?}; contract: {:?}; clarity_version: {:?}", tx.txid(), contract_id, clarity_version);

                        let db_contract = app_db
                            .insert_contract(db_block.id, &contract_id.to_string(), &contract.code_body.to_string())?;

                        app_db.insert_execution(db_block.id, &tx.txid().0, db_contract.id)?;
                    },
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
    })?;
    ok!()
}
