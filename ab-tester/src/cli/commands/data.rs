use std::{time::Duration, collections::HashMap};

use color_eyre::eyre::Result;
use diesel::{Connection, SqliteConnection};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress, WeakProgressBar};
use log::*;

use crate::{
    cli::{DataArgs, console::app},
    context::{
        callbacks::{ReplayCallbacks, RuntimeEnvCallbacks}, environments::{RuntimeEnvBuilder, RuntimeEnv}, replay::ReplayOpts,
        ComparisonContext, Network, Runtime,
    },
    db::appdb::AppDb,
    ok,
};

pub async fn exec(config: &crate::config::Config, data_args: DataArgs) -> Result<()> {
    let app_db_conn = SqliteConnection::establish(&config.app.db_path)?;
    let app_db = AppDb::new(app_db_conn);

    let env_builder = RuntimeEnvBuilder::new(&app_db);

    let mut baseline_env = env_builder.stacks_node(
        "baseline", 
        &config.baseline.chainstate_path
    )?;

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

    let multi_pb = MultiProgress::new();
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            // For more spinners check out the cli-spinners project:
            // https://github.com/sindresorhus/cli-spinners/blob/master/spinners.json
            .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏",]),
    );

    let mut progress_bars: HashMap<&str, WeakProgressBar> = HashMap::new();

    /*
    let baseline_env_cb = RuntimeEnvCallbacks::default();
    //baseline_env.with_callbacks(&baseline_env_cb);
    let baseline_env_pg = ProgressBar::new(512);
    progress_bars.insert(baseline_env.name(), baseline_env_pg.downgrade());
    multi_pb.add(baseline_env_pg);

    let interpreter_env_cb = RuntimeEnvCallbacks::default();
    interpreter_env.with_callbacks(&interpreter_env_cb);
    let interpreter_env_pb = ProgressBar::new(512);
    progress_bars.insert(interpreter_env.name(), interpreter_env_pb.downgrade());
    multi_pb.add(interpreter_env_pb);

    let wasm_env_cb = RuntimeEnvCallbacks::default();
    wasm_env.with_callbacks(&wasm_env_cb);
    let wasm_env_pb = ProgressBar::new(512);
    progress_bars.insert(wasm_env.name(), wasm_env_pb.downgrade());
    multi_pb.add(wasm_env_pb);

    
    let mut replay_opts: ReplayOpts = data_args.into();
    replay_opts.with_callbacks(ReplayCallbacks { 
        replay_start: &|_,_,_| {}, 
        replay_finish: &|_,_| {}, 
        replay_block_start: &|_,_,_,_| {}, 
        replay_block_finish: &|_,_| {}, 
        replay_tx_start: &|_,_| {}, 
        replay_tx_finish: &|_,_| {} 
    });
*/
    let tmp = ComparisonContext::new(&app_db)
        .using_baseline(&mut baseline_env);


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
