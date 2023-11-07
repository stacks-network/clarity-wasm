use color_eyre::eyre::Result;
use diesel::{Connection, SqliteConnection};
use log::*;

use crate::{
    cli::DataArgs,
    context::{
        environments::{ReadableEnv, RuntimeEnvBuilder},
        Block, ComparisonContext, Network, Runtime,
    },
    db::appdb::AppDb,
    ok,
};

pub async fn exec(config: &crate::config::Config, data_args: DataArgs) -> Result<()> {
    let app_db_conn = SqliteConnection::establish(&config.app.db_path)?;
    let app_db = AppDb::new(app_db_conn);

    let env_builder = RuntimeEnvBuilder::new(&app_db);

    let baseline_env =
        env_builder.stacks_node("baseline", &config.baseline.chainstate_path)?;

    let mut interpreter_env = env_builder.instrumented(
        "baseline_replay",
        Runtime::Interpreter,
        Network::Mainnet(1),
        "/home/cylwit/clarity-ab/replay",
    )?;

    let mut wasm_env = env_builder.instrumented(
        "wasm_env",
        Runtime::Wasm,
        Network::Mainnet(1),
        "/home/cylwit/clarity-ab/wasm",
    )?;

    let comparator = ComparisonContext::new(&app_db)
        .using_baseline(&baseline_env)
        .instrument_into(&mut interpreter_env)
        .instrument_into(&mut wasm_env)
        .replay()?;

    std::process::exit(0);

    info!(
        "aggregating contract calls starting at block height {}...",
        data_args.from_height
    );

    let mut processed_block_count = 0;

    for block in baseline_env.blocks()?.into_iter() {
        let (header, stacks_block) = match &block {
            Block::Boot(header) => {
                // We can't process the boot block, so skip it.
                info!("boot block - skipping '{:?}'", header.index_block_hash);
                continue;
            }
            Block::Genesis(header) => {
                // We can't process genesis (doesn't exist in chainstate), so skip it.
                //info!("genesis block - skipping '{:?}'", gen.index_block_hash);
                //continue;
                info!("genesis block");
                (header, None)
            }
            Block::Regular(header, block) => (header, Some(block)),
        };

        // Ensure that we've reached the specified block-height before beginning
        // processing.
        if header.block_height() < data_args.from_height {
            continue;
        }

        // Ensure that we haven't exceeded the specified max-blocks for processing.
        data_args.assert_max_processed_block_count(processed_block_count)?;

        // Ensure that we haven't reached the specified max block-height for processing.
        data_args.assert_block_height_under_max_height(header.block_height())?;

        debug!(
            "processing block #{} ({})",
            header.block_height(),
            &hex::encode(&header.index_block_hash)
        );
        /*replay_env_mut.block_begin(&block, |_ctx| {
            info!("processing block!");
            ok!()
        })?;*/

        processed_block_count += 1;
        continue;

        /*let block_id = header.stacks_block_id()?;

        for tx in stacks_block.unwrap().txs.iter() {
            info!("processing tx: {}", tx.txid());

            let origin_principal = StandardPrincipalData::from(tx.origin_address());

            #[allow(clippy::single_match)]
            match &tx.payload {
                TransactionPayload::SmartContract(contract, _) => {
                    let contract_id =
                        QualifiedContractIdentifier::new(origin_principal, contract.name.clone());

                    if let Some(entry) = contracts.get(&contract_id) {
                        warn!(
                            "duplicate: {}, first block={}, second block={}",
                            contract_id, entry, &block_id
                        );
                    } else {
                        contracts.insert(contract_id, block_id);
                    }
                }
                _ => {}
            }
        }*/
    }

    info!("blocks processed: {processed_block_count}");

    std::process::exit(1);

    //crate::runtime::analyze_contract(contract_identifier, expressions, data_store, cost_tracker);
    //crate::runtime::install_contract(contract_identifier, expressions, clarity_db, cost_tracker);
    /*
    let mut baseline_env = TestEnv::new(
        "baseline",
        &config.baseline.chainstate_path,
        &mut app_db)?;

    let wasm_env = TestEnv::new(
        "wasm",
        &config.envs("wasm").chainstate_path,
        &mut app_db)?;

    baseline_env.with_env(|ctx| {
        let mut contract_calls: Vec<TransactionContractCall> = Default::default();

        info!(
            "aggregating contract calls starting at block height {}...",
            data_args.from_height
        );
        let mut processed_block_count = 0;
        for block_header in ctx.blocks(data_args.from_height)? {
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
            let block = ctx.get_stacks_block(block_header.index_block_hash())?;

            // Load the block
            debug!("loading block '{block_id}'");
            ctx.load_block(&block_id)?;

            debug!("inserting block into app db");
            let db_block = ctx.app_db().insert_block(
                ctx.env_id(),
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

                        /*let db_contract = ctx.db()
                            .insert_contract(db_block.id, &contract_id.to_string(), &contract.code_body.to_string())?;

                        app_db.insert_execution(db_block.id, &tx.txid().0, db_contract.id)?;*/

                        /*ctx.with_app_db(|db| {
                            let db_contract = db.insert_contract(db_block.id, &contract_id.to_string(), &contract.code_body.to_string())?;
                            db.insert_execution(db_block.id, &tx.txid().0, db_contract.id)?;
                            ok!()
                        })?;*/

                        //StacksChainState::process_transaction_payload(clarity_tx, tx, &tx., ASTRules::Typical);
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

    &baseline_env.db();*/

    ok!()
}
