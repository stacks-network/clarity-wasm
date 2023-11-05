use blockstack_lib::{chainstate::stacks::db::StacksChainState, core::BLOCK_LIMIT_MAINNET_205};
use clarity::vm::{
    analysis::{run_analysis, AnalysisDatabase, ContractAnalysis},
    ast::{self, ASTRules},
    contexts::GlobalContext,
    costs::LimitedCostTracker,
    database::{
        ClarityBackingStore, ClarityDatabase, RollbackWrapper, NULL_BURN_STATE_DB, NULL_HEADER_DB,
    },
    eval_all,
    types::QualifiedContractIdentifier,
    ClarityVersion, ContractContext, SymbolicExpression,
};
use color_eyre::Result;
use stacks_common::types::{
    chainstate::{BlockHeaderHash, ConsensusHash},
    StacksEpochId,
};

use crate::{
    config::Config, db::appdb::AppDb, db::datastore::DataStore,
    db::model::app_db::ContractExecution,
};

pub fn analyze_contract(
    contract_identifier: &QualifiedContractIdentifier,
    expressions: &mut [SymbolicExpression],
    data_store: &mut dyn ClarityBackingStore,
    cost_tracker: LimitedCostTracker,
) -> Result<ContractAnalysis> {
    let mut analysis_db = AnalysisDatabase::new(data_store);

    let contract_analysis = run_analysis(
        contract_identifier,
        expressions,
        &mut analysis_db,
        false,
        cost_tracker,
        StacksEpochId::latest(),
        ClarityVersion::latest(),
    )
    .expect("contract analysis failed");

    Ok(contract_analysis)
}

pub fn clarity_tx() {
    let (mut chainstate, _) = StacksChainState::open_and_exec(true, 1, "path", None, None)
        .expect("failed to open chainstate");

    let parent_consensus_hash = ConsensusHash::from_bytes(&[1, 2, 3, 4]).unwrap();
    let new_consensus_hash = ConsensusHash::from_bytes(&[5, 6, 7, 8]).unwrap();
    let parent_block = BlockHeaderHash::from_bytes(&[1, 2, 3, 4]).unwrap();
    let new_block = BlockHeaderHash::from_bytes(&[5, 6, 7, 8]).unwrap();

    let mut clarity_tx = chainstate.block_begin(
        &NULL_BURN_STATE_DB,
        &parent_consensus_hash,
        &parent_block,
        &new_consensus_hash,
        &new_block,
    );

    //StacksChainState::process_transaction(&mut clarity_tx, tx, quiet, ast_rules);
}

pub fn install_contract(
    contract_identifier: &QualifiedContractIdentifier,
    expressions: &[SymbolicExpression],
    clarity_db: ClarityDatabase,
    cost_tracker: LimitedCostTracker,
) -> Result<()> {
    let mut contract_context =
        ContractContext::new(contract_identifier.clone(), ClarityVersion::latest());

    let mut global_context =
        GlobalContext::new(true, 1, clarity_db, cost_tracker, StacksEpochId::latest());

    global_context.execute(|ctx| eval_all(expressions, &mut contract_context, ctx, None))?;

    Ok(())
}

fn clarity_db_for_execution<'a>(
    execution: ContractExecution,
    db: &'a mut AppDb,
    data_store: &'a mut DataStore,
) -> Result<ClarityDatabase<'a>> {
    let rollback_wrapper = RollbackWrapper::new(data_store);
    let mut clarity_db = ClarityDatabase::new_with_rollback_wrapper(
        rollback_wrapper,
        &NULL_HEADER_DB,
        &NULL_BURN_STATE_DB,
    );

    clarity_db.begin();
    clarity_db.set_clarity_epoch_version(StacksEpochId::latest());
    clarity_db.commit();

    Ok(clarity_db)
}

fn exec<'a>(
    config: &'a Config,
    db: &'a mut AppDb,
    block_id: i32,
    transaction_id: &[u8],
    source_code: &str,
    contract_identifier: &QualifiedContractIdentifier,
    clarity_version: ClarityVersion,
) -> Result<()> {
    let contract_id = db
        .get_contract_id(contract_identifier)
        .expect("failed to execute query")
        .expect("contract not found");

    let execution = db
        .insert_execution(block_id, &transaction_id, contract_id)
        .expect("failed to execute query");

    let mut data_store = DataStore::new(db);
    let rollback_wrapper = RollbackWrapper::new(&mut data_store);
    let mut clarity_db = ClarityDatabase::new_with_rollback_wrapper(
        rollback_wrapper,
        &NULL_HEADER_DB,
        &NULL_BURN_STATE_DB,
    );
    clarity_db.begin();
    clarity_db.set_clarity_epoch_version(StacksEpochId::latest());
    clarity_db.commit();

    let mut cost_tracker = LimitedCostTracker::new(
        true,
        1,
        BLOCK_LIMIT_MAINNET_205,
        &mut clarity_db,
        StacksEpochId::Epoch21,
    )
    .expect("failed to create cost tracker");

    let mut ast = ast::build_ast_with_rules(
        &contract_identifier,
        source_code,
        &mut cost_tracker,
        clarity_version,
        StacksEpochId::latest(),
        ASTRules::Typical,
    )
    .expect("failed to parse ast");

    let contract_analysis = analyze_contract(
        contract_identifier,
        &mut ast.expressions,
        &mut data_store,
        cost_tracker,
    );

    let (mut chainstate, _) = StacksChainState::open_and_exec(
        true,
        config.baseline.chain_id,
        &config.baseline.chainstate_path,
        None,
        None,
    )
    .expect("failed to open/init chainstate");

    todo!()
}
