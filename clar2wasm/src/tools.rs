//! The `tools` module contains tools for evaluating Clarity snippets.
//! It is intended for use in tooling and tests, but not intended to be used
//! in production.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;

use clarity::consts::CHAIN_ID_TESTNET;
use clarity::types::StacksEpochId;
use clarity::vm::analysis::run_analysis;
use clarity::vm::ast::build_ast;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::contracts::Contract;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::ClarityDatabase;
use clarity::vm::errors::{CheckErrors, Error, WasmError};
use clarity::vm::types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData};
use clarity::vm::{eval_all, ClarityVersion, ContractContext, ContractName, Value};

use crate::compile;
use crate::datastore::{BurnDatastore, Datastore, StacksConstants};
use crate::initialize::initialize_contract;

#[derive(Clone)]
pub struct TestEnvironment {
    contract_contexts: HashMap<String, ContractContext>,
    epoch: StacksEpochId,
    version: ClarityVersion,
    datastore: Datastore,
    burn_datastore: BurnDatastore,
    cost_tracker: LimitedCostTracker,
}

impl TestEnvironment {
    pub fn new_with_amount(amount: u128, epoch: StacksEpochId, version: ClarityVersion) -> Self {
        let constants = StacksConstants::default();
        let burn_datastore = BurnDatastore::new(constants.clone());
        let mut datastore = Datastore::new();
        let cost_tracker = LimitedCostTracker::new_free();

        let mut db = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
        db.begin();
        db.set_clarity_epoch_version(epoch)
            .expect("Failed to set epoch version.");
        db.commit().expect("Failed to commit.");

        // Give one account a starting balance, to be used for testing.
        let recipient = PrincipalData::Standard(StandardPrincipalData::transient());
        let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
        execute(&mut conn, |database| {
            let mut snapshot = database.get_stx_balance_snapshot(&recipient)?;
            snapshot.credit(amount)?;
            snapshot.save()?;
            database.increment_ustx_liquid_supply(amount)
        })
        .expect("Failed to increment liquid supply.");

        Self {
            contract_contexts: HashMap::new(),
            epoch,
            version,
            datastore,
            burn_datastore,
            cost_tracker,
        }
    }

    pub fn new(epoch: StacksEpochId, version: ClarityVersion) -> Self {
        Self::new_with_amount(1_000_000_000, epoch, version)
    }

    pub fn init_contract_with_snippet(
        &mut self,
        contract_name: &str,
        snippet: &str,
    ) -> Result<Option<Value>, Error> {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            (*contract_name).into(),
        );

        let mut compile_result = self
            .datastore
            .as_analysis_db()
            .execute(|analysis_db| {
                compile(
                    snippet,
                    &contract_id,
                    LimitedCostTracker::new_free(),
                    self.version,
                    self.epoch,
                    analysis_db,
                )
                .map_err(|e| CheckErrors::Expects(format!("Compilation failure {:?}", e)))
            })
            .map_err(|e| Error::Wasm(WasmError::WasmGeneratorError(format!("{:?}", e))))?;

        self.datastore
            .as_analysis_db()
            .execute(|analysis_db| {
                analysis_db.insert_contract(&contract_id, &compile_result.contract_analysis)
            })
            .expect("Failed to insert contract analysis.");

        let mut contract_context = ContractContext::new(contract_id.clone(), self.version);
        // compile_result.module.emit_wasm_file("test.wasm").unwrap();
        contract_context.set_wasm_module(compile_result.module.emit_wasm());

        let mut cost_tracker = LimitedCostTracker::new_free();
        std::mem::swap(&mut self.cost_tracker, &mut cost_tracker);

        let conn = ClarityDatabase::new(
            &mut self.datastore,
            &self.burn_datastore,
            &self.burn_datastore,
        );
        let mut global_context =
            GlobalContext::new(false, CHAIN_ID_TESTNET, conn, cost_tracker, self.epoch);
        global_context.begin();
        global_context
            .execute(|g| g.database.insert_contract_hash(&contract_id, snippet))
            .expect("Failed to insert contract hash.");

        let return_val = initialize_contract(
            &mut global_context,
            &mut contract_context,
            None,
            &compile_result.contract_analysis,
        )?;

        let data_size = contract_context.data_size;
        global_context.database.insert_contract(
            &contract_id,
            Contract {
                contract_context: contract_context.clone(),
            },
        )?;
        global_context
            .database
            .set_contract_data_size(&contract_id, data_size)
            .expect("Failed to set contract data size.");

        global_context.commit().unwrap();
        self.cost_tracker = global_context.cost_track;

        self.contract_contexts
            .insert(contract_name.to_string(), contract_context);

        Ok(return_val)
    }

    pub fn evaluate(&mut self, snippet: &str) -> Result<Option<Value>, Error> {
        self.init_contract_with_snippet("snippet", snippet)
    }

    pub fn get_contract_context(&self, contract_name: &str) -> Option<&ContractContext> {
        self.contract_contexts.get(contract_name)
    }

    pub fn advance_chain_tip(&mut self, count: u32) -> u32 {
        self.burn_datastore.advance_chain_tip(count);
        self.datastore.advance_chain_tip(count)
    }

    pub fn interpret_contract_with_snippet(
        &mut self,
        contract_name: &str,
        snippet: &str,
    ) -> Result<Option<Value>, Error> {
        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            (*contract_name).into(),
        );

        let mut cost_tracker = LimitedCostTracker::new_free();
        std::mem::swap(&mut self.cost_tracker, &mut cost_tracker);

        let mut contract_analysis = self.datastore.as_analysis_db().execute(|analysis_db| {
            // Parse the contract
            let ast = build_ast(
                &contract_id,
                snippet,
                &mut self.cost_tracker,
                self.version,
                self.epoch,
            )
            .map_err(|e| Error::Wasm(WasmError::WasmGeneratorError(format!("{:?}", e))))?;

            // Run the analysis passes
            run_analysis(
                &contract_id,
                &ast.expressions,
                analysis_db,
                false,
                cost_tracker,
                self.epoch,
                self.version,
                true,
            )
            .map_err(|(e, _)| Error::Wasm(WasmError::WasmGeneratorError(format!("{:?}", e))))
        })?;

        self.datastore
            .as_analysis_db()
            .execute(|analysis_db| analysis_db.insert_contract(&contract_id, &contract_analysis))
            .expect("Failed to insert contract analysis");

        let mut contract_context = ContractContext::new(contract_id.clone(), self.version);

        let conn = ClarityDatabase::new(
            &mut self.datastore,
            &self.burn_datastore,
            &self.burn_datastore,
        );

        let mut global_context = GlobalContext::new(
            false,
            CHAIN_ID_TESTNET,
            conn,
            contract_analysis.cost_track.take().unwrap(),
            self.epoch,
        );

        global_context.begin();

        global_context
            .database
            .insert_contract_hash(&contract_id, snippet)
            .expect("Failed to insert contract hash.");

        let result = eval_all(
            &contract_analysis.expressions,
            &mut contract_context,
            &mut global_context,
            None,
        )?;

        global_context.database.insert_contract(
            &contract_id,
            Contract {
                contract_context: contract_context.clone(),
            },
        )?;
        global_context
            .database
            .set_contract_data_size(&contract_id, contract_context.data_size)
            .expect("Failed to set contract data size.");

        global_context.commit().unwrap();
        self.cost_tracker = global_context.cost_track;

        self.contract_contexts
            .insert(contract_name.to_owned(), contract_context);

        Ok(result)
    }

    pub fn interpret(&mut self, snippet: &str) -> Result<Option<Value>, Error> {
        self.interpret_contract_with_snippet("snippet", snippet)
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new(StacksEpochId::Epoch25, ClarityVersion::Clarity2)
    }
}

pub fn execute<F, T, E>(conn: &mut ClarityDatabase, f: F) -> std::result::Result<T, E>
where
    F: FnOnce(&mut ClarityDatabase) -> std::result::Result<T, E>,
{
    conn.begin();
    let result = f(conn).map_err(|e| {
        conn.roll_back().expect("Failed to roll back");
        e
    })?;
    conn.commit().expect("Failed to commit");
    Ok(result)
}

/// Evaluate a Clarity snippet at a specific epoch and version.
/// Returns an optional value -- the result of the evaluation.
pub fn evaluate_at(
    snippet: &str,
    epoch: StacksEpochId,
    version: ClarityVersion,
) -> Result<Option<Value>, Error> {
    let mut env = TestEnvironment::new(epoch, version);
    env.evaluate(snippet)
}

/// Evaluate a Clarity snippet at a specific epoch and version, with a default
/// amount of money for the transient principal account.
/// Returns an optional value -- the result of the evaluation.
pub fn evaluate_at_with_amount(
    snippet: &str,
    amount: u128,
    epoch: StacksEpochId,
    version: ClarityVersion,
) -> Result<Option<Value>, Error> {
    let mut env = TestEnvironment::new_with_amount(amount, epoch, version);
    env.evaluate(snippet)
}

/// Evaluate a Clarity snippet at the latest epoch and clarity version.
/// Returns an optional value -- the result of the evaluation.
#[allow(clippy::result_unit_err)]
pub fn evaluate(snippet: &str) -> Result<Option<Value>, Error> {
    evaluate_at(snippet, StacksEpochId::latest(), ClarityVersion::latest())
}

/// Interpret a Clarity snippet at a specific epoch and version.
/// Returns an optional value -- the result of the evaluation.
pub fn interpret_at(
    snippet: &str,
    epoch: StacksEpochId,
    version: ClarityVersion,
) -> Result<Option<Value>, Error> {
    let mut env = TestEnvironment::new(epoch, version);
    env.interpret(snippet)
}

/// Interpret a Clarity snippet at a specific epoch and version, with a default
/// amount of money for the transient principal account.
/// Returns an optional value -- the result of the evaluation.
pub fn interpret_at_with_amount(
    snippet: &str,
    amount: u128,
    epoch: StacksEpochId,
    version: ClarityVersion,
) -> Result<Option<Value>, Error> {
    let mut env = TestEnvironment::new_with_amount(amount, epoch, version);
    env.interpret(snippet)
}

/// Interprets a Clarity snippet at the latest epoch and clarity version.
/// Returns an optional value -- the result of the evaluation.
#[allow(clippy::result_unit_err)]
pub fn interpret(snippet: &str) -> Result<Option<Value>, Error> {
    interpret_at(snippet, StacksEpochId::latest(), ClarityVersion::latest())
}

struct TestConfig;

impl TestConfig {
    /// Select a Clarity version based on enabled features.
    pub fn clarity_version() -> ClarityVersion {
        match () {
            _ if cfg!(feature = "test-clarity-v1") => ClarityVersion::Clarity1,
            _ if cfg!(feature = "test-clarity-v2") => ClarityVersion::Clarity2,
            _ if cfg!(feature = "test-clarity-v3") => ClarityVersion::Clarity3,
            _ => ClarityVersion::latest(),
        }
    }

    /// Latest epoch for the enabled Clarity version.
    pub fn latest_epoch_for_clarity_version() -> StacksEpochId {
        match TestConfig::clarity_version() {
            ClarityVersion::Clarity1 => StacksEpochId::Epoch2_05,
            ClarityVersion::Clarity2 => StacksEpochId::Epoch25,
            _ => StacksEpochId::latest(),
        }
    }
}

pub fn crosscheck(snippet: &str, expected: Result<Option<Value>, Error>) {
    let compiled = evaluate_at(
        snippet,
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version(),
    );

    let interpreted = interpret_at(
        snippet,
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version(),
    );

    assert_eq!(
        compiled, interpreted,
        "Compiled and interpreted results diverge!\ncompiled: {:?}\ninterpreted: {:?}",
        &compiled, &interpreted
    );

    assert_eq!(
        compiled, expected,
        "value is not the expected {:?}",
        compiled
    );
}

pub fn crosscheck_with_amount(snippet: &str, amount: u128, expected: Result<Option<Value>, ()>) {
    let compiled = evaluate_at_with_amount(
        snippet,
        amount,
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version(),
    );

    let interpreted = interpret_at_with_amount(
        snippet,
        amount,
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version(),
    );

    assert_eq!(
        compiled.as_ref().map_err(|_| &()),
        interpreted.as_ref().map_err(|_| &()),
        "Compiled and interpreted results diverge!\ncompiled: {:?}\ninterpreted: {:?}",
        &compiled,
        &interpreted
    );

    assert_eq!(
        compiled.as_ref().map_err(|_| &()),
        expected.as_ref(),
        "value is not the expected {:?}",
        compiled
    );
}

pub fn crosscheck_compare_only(snippet: &str) {
    // to avoid false positives when both the compiled and interpreted fail,
    // we don't allow failures in these tests

    // Note that we interpret first, to catch logical errors early

    let interpreted = interpret(snippet).expect("Interpreted snippet failed");
    let compiled = evaluate(snippet).expect("Compiled snippet failed");

    assert_eq!(
        compiled, interpreted,
        "Compiled and interpreted results diverge! {}\ncompiled: {:?}\ninterpreted: {:?}",
        snippet, &compiled, &interpreted
    );
}

pub fn crosscheck_compare_only_with_expected_error<E: Fn(&Error) -> bool>(
    snippet: &str,
    expected: E,
) {
    let compiled = evaluate(snippet);
    let interpreted = interpret(snippet);

    if let Err(e) = &compiled {
        if !expected(e) {
            panic!("Compiled snippet failed with unexpected error: {:?}", e);
        }
    }

    assert_eq!(
        compiled, interpreted,
        "Compiled and interpreted results diverge! {}\ncompiled: {:?}\ninterpreted: {:?}",
        snippet, &compiled, &interpreted
    );
}

/// Advance the block height to `count`, and uses identical TestEnvironment copies
/// to assert the results of a contract snippet running against the compiler and the interpreter.
pub fn crosscheck_compare_only_advancing_tip(snippet: &str, count: u32) {
    let mut compiler_env = TestEnvironment::new(
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version(),
    );
    compiler_env.advance_chain_tip(count);

    let mut interpreter_env = compiler_env.clone();

    let compiled = compiler_env.evaluate(snippet).map_err(|_| ());
    let interpreted = interpreter_env.interpret(snippet).map_err(|_| ());

    assert_eq!(
        compiled, interpreted,
        "Compiled and interpreted results diverge! {}\ncompiled: {:?}\ninterpreted: {:?}",
        snippet, &compiled, &interpreted
    );
}

pub fn crosscheck_with_epoch(
    snippet: &str,
    expected: Result<Option<Value>, Error>,
    epoch: StacksEpochId,
) {
    let clarity_version = ClarityVersion::default_for_epoch(epoch);
    let compiled = evaluate_at(snippet, epoch, clarity_version);
    let interpreted = interpret_at(snippet, epoch, clarity_version);

    assert_eq!(
        compiled, interpreted,
        "Compiled and interpreted results diverge!\ncompiled: {:?}\ninterpreted: {:?}",
        &compiled, &interpreted
    );

    assert_eq!(
        compiled, expected,
        "value is not the expected {:?}",
        compiled
    );
}

pub fn crosscheck_validate<V: Fn(Value)>(snippet: &str, validator: V) {
    let compiled = evaluate_at(
        snippet,
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version(),
    );

    let interpreted = interpret_at(
        snippet,
        TestConfig::latest_epoch_for_clarity_version(),
        TestConfig::clarity_version()
    );

    assert_eq!(
        compiled, interpreted,
        "Compiled and interpreted results diverge! {}\ncompiled: {:?}\ninterpreted: {:?}",
        snippet, &compiled, &interpreted
    );

    let value = compiled.unwrap().unwrap();
    validator(value)
}

pub fn crosscheck_multi_contract(
    contracts: &[(ContractName, &str)],
    expected: Result<Option<Value>, Error>,
) {
    // compiled version
    let mut env = TestEnvironment::default();
    let compiled_results: Vec<_> = contracts
        .iter()
        .map(|(name, snippet)| env.init_contract_with_snippet(name, snippet))
        .collect();

    // interpreted version
    let mut env = TestEnvironment::default();
    let interpreted_results = contracts
        .iter()
        .map(|(name, snippet)| env.interpret_contract_with_snippet(name, snippet));

    // compare results contract by contract
    for ((cmp_res, int_res), (contract_name, _)) in compiled_results
        .iter()
        .zip(interpreted_results)
        .zip(contracts)
    {
        assert_eq!(
            cmp_res, &int_res,
            "Compiled and interpreted results diverge in contract \"{contract_name}\"\ncompiled: {cmp_res:?}\ninterpreted: {int_res:?}"
        );
    }

    // compare with expected final value
    let final_value = compiled_results.last().unwrap_or(&Ok(None));
    assert_eq!(
        final_value, &expected,
        "final value is not the expected {final_value:?}"
    );
}

// TODO: This function is a temporary solution until issue #421 is addressed.
// Tests that call this function will need to be adjusted.
//
// Consider gating tests to epochs whenever possible
// using the `crosscheck_with_epoch` function.
pub fn crosscheck_expect_failure(snippet: &str) {
    let compiled = evaluate(snippet);
    let interpreted = interpret(snippet);

    assert_eq!(
        compiled.is_err(),
        interpreted.is_err(),
        "Compiled and interpreted results diverge! {}\ncompiled: {:?}\ninterpreted: {:?}",
        snippet,
        &compiled,
        &interpreted
    );
}

#[test]
fn test_evaluate_snippet() {
    assert_eq!(evaluate("(+ 1 2)"), Ok(Some(Value::Int(3))));
}
