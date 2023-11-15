use std::rc::Rc;
use std::time::Duration;

use color_eyre::eyre::Result;
use diesel::{Connection, SqliteConnection};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::cli::DataArgs;
use crate::context::replay::ReplayOpts;
use crate::context::{ComparisonContext, Network, Runtime};
use crate::db::appdb::AppDb;
use crate::ok;

pub async fn exec(config: &crate::config::Config, data_args: DataArgs) -> Result<()> {
    let app_db_conn = SqliteConnection::establish(&config.app.db_path)?;
    let app_db = Rc::new(AppDb::new(app_db_conn));

    let _multi_pb = MultiProgress::new();
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    let replay_opts: ReplayOpts = data_args.into();

    let _compare_ctx = ComparisonContext::new(app_db.clone())
        .using_baseline(|from| from.stacks_node("baseline", &config.baseline.chainstate_path))?
        .instrument_into(|into| {
            into.instrumented(
                "interpreter_replay",
                Runtime::Interpreter,
                Network::Mainnet(1),
                "/home/cylwit/clarity-ab/replay",
            )?
            .instrumented(
                "wasm",
                Runtime::Wasm,
                Network::Mainnet(1),
                "/home/cylwit/clarity-ab/wasm",
            )
        })?
        .replay(&replay_opts)?;
    //.instrument_into(&mut interpreter_env)
    //.instrument_into(&mut wasm_env);

    std::process::exit(0);

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
