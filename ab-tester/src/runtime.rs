use color_eyre::Result;
use blockstack_lib::{chainstate::stacks::db::StacksChainState, core::BLOCK_LIMIT_MAINNET_205};
use clarity::vm::{
    ClarityVersion, ContractContext, 
    types::QualifiedContractIdentifier, 
    database::{NULL_HEADER_DB, NULL_BURN_STATE_DB, RollbackWrapper, ClarityDatabase, ClarityBackingStore}, 
    contexts::GlobalContext, 
    costs::LimitedCostTracker, 
    ast::{self, ASTRules}, eval_all, analysis::{AnalysisDatabase, run_analysis, ContractAnalysis}, SymbolicExpression};
use stacks_common::types::StacksEpochId;

use crate::{config::Config, datastore::DataStore, appdb::AppDb, model::app_db::ContractExecution};


pub fn analyze_contract(
    contract_identifier: &QualifiedContractIdentifier, 
    expressions: &mut [SymbolicExpression],
    data_store: &mut dyn ClarityBackingStore,
    cost_tracker: LimitedCostTracker
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
    ).expect("contract analysis failed");

    Ok(contract_analysis)
}

pub fn install_contract(
    contract_identifier: &QualifiedContractIdentifier, 
    expressions: &[SymbolicExpression], 
    clarity_db: ClarityDatabase, 
    cost_tracker: LimitedCostTracker
) -> Result<()> {
    let mut contract_context = ContractContext::new(
        contract_identifier.clone(), ClarityVersion::latest());

    let mut global_context = GlobalContext::new(
        true, 
        1, 
        clarity_db, 
        cost_tracker,
        StacksEpochId::latest());

    global_context.execute(|ctx| {
        eval_all(
            expressions,
            &mut contract_context,
            ctx,
            None,
        )
    })?;

    Ok(())
}

fn clarity_db_for_execution<'a>(
    execution: ContractExecution, 
    db: &'a mut AppDb,
    data_store: &'a mut DataStore
) -> Result<ClarityDatabase<'a>> {
    
    let rollback_wrapper = RollbackWrapper::new(data_store);
    let mut clarity_db = ClarityDatabase::new_with_rollback_wrapper(
        rollback_wrapper, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);
        
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
    clarity_version: ClarityVersion
) -> Result<()> {
    let contract_id = db.get_contract_id(contract_identifier)
        .expect("failed to execute query")
        .expect("contract not found");

    let execution = db
        .insert_execution(block_id, &transaction_id, contract_id)
        .expect("failed to execute query");

    let mut data_store = DataStore::new(db);
    let rollback_wrapper = RollbackWrapper::new(&mut data_store);
    let mut clarity_db = ClarityDatabase::new_with_rollback_wrapper(
        rollback_wrapper, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);
    clarity_db.begin();
    clarity_db.set_clarity_epoch_version(StacksEpochId::latest());
    clarity_db.commit();

    let mut cost_tracker = LimitedCostTracker::new(
        true, 1, BLOCK_LIMIT_MAINNET_205, &mut clarity_db, StacksEpochId::Epoch21)
        .expect("failed to create cost tracker");

    let mut ast = ast::build_ast_with_rules(
        &contract_identifier, source_code, &mut cost_tracker, clarity_version, StacksEpochId::latest(), ASTRules::Typical)
        .expect("failed to parse ast");

    let contract_analysis = analyze_contract(contract_identifier, &mut ast.expressions, &mut data_store, cost_tracker);

    let (mut chainstate, _) = StacksChainState::open_and_exec(
        true,
        config.baseline.chain_id,
        &config.baseline.chainstate_path,
        None,
    None)
        .expect("failed to open/init chainstate");

    

    todo!()
}