//! The `tools` module contains tools for evaluating Clarity snippets.
//! It is intended for use in tooling and tests, but not intended to be used
//! in production. The `tools` module is only available when the
//! `developer-mode` feature is enabled.

use std::collections::{BTreeMap, HashMap};

use clarity::consts::CHAIN_ID_TESTNET;
use clarity::types::StacksEpochId;
use clarity::vm::clarity_wasm::initialize_contract;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::contracts::Contract;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::ClarityDatabase;
use clarity::vm::errors::{Error, WasmError};
use clarity::vm::types::{
    CharType, ListData, OptionalData, PrincipalData, QualifiedContractIdentifier, ResponseData,
    SequenceData, StandardPrincipalData, TupleData, UTF8Data,
};
use clarity::vm::{execute_v2 as execute, ClarityVersion, ContractContext, Value};

use crate::compile;
use crate::datastore::{BurnDatastore, Datastore, StacksConstants};

pub struct TestEnvironment {
    contract_contexts: HashMap<String, ContractContext>,
    epoch: StacksEpochId,
    version: ClarityVersion,
    datastore: Datastore,
    burn_datastore: BurnDatastore,
    cost_tracker: LimitedCostTracker,
}

impl TestEnvironment {
    pub fn new(epoch: StacksEpochId, version: ClarityVersion) -> Self {
        let constants = StacksConstants::default();
        let burn_datastore = BurnDatastore::new(constants.clone());
        let mut datastore = Datastore::new();
        let cost_tracker = LimitedCostTracker::new_free();

        let mut db = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
        db.begin();
        db.set_clarity_epoch_version(epoch);
        db.commit();

        // Give one account a starting balance, to be used for testing.
        let recipient = PrincipalData::Standard(StandardPrincipalData::transient());
        let amount = 1_000_000_000;
        let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
        conn.execute(|database| {
            let mut snapshot = database.get_stx_balance_snapshot(&recipient);
            snapshot.credit(amount);
            snapshot.save();
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
        );
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
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new(StacksEpochId::latest(), ClarityVersion::latest())
    }
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

/// Evaluate a Clarity snippet at the latest epoch and clarity version.
/// Returns an optional value -- the result of the evaluation.
#[allow(clippy::result_unit_err)]
pub fn evaluate(snippet: &str) -> Result<Option<Value>, ()> {
    evaluate_at(snippet, StacksEpochId::latest(), ClarityVersion::latest()).map_err(|_| ())
}
pub fn unicode_to_byte_sequence(input: Value) -> Value {
    match &input {
        Value::Sequence(SequenceData::List(list_data)) => {
            let mut processed_data = Vec::new();

            for val in &list_data.data {
                processed_data.push(unicode_to_byte_sequence(val.clone()));
            }

            Value::Sequence(SequenceData::List(ListData {
                data: processed_data,
                type_signature: list_data.type_signature.clone(),
            }))
        }
        Value::Response(response_data) => Value::Response(ResponseData {
            committed: response_data.committed,
            data: Box::new(unicode_to_byte_sequence(*(response_data.data).clone())),
        }),
        Value::Optional(optional_data) => {
            if let Some(data) = &optional_data.data {
                Value::Optional(OptionalData {
                    data: Some(Box::new(unicode_to_byte_sequence(*data.clone()))),
                })
            } else {
                Value::Optional(optional_data.clone())
            }
        }
        Value::Tuple(tuple_data) => {
            let mut btree_map = BTreeMap::new();
            for (name, value) in tuple_data.data_map.iter() {
                btree_map.insert(name.clone(), unicode_to_byte_sequence(value.clone()));
            }
            Value::Tuple(TupleData {
                data_map: btree_map,
                type_signature: tuple_data.type_signature.clone(),
            })
        }
        Value::Sequence(SequenceData::String(CharType::UTF8(u))) => {
            println!("Input Data :{:?}", u.data);

            let scalar_bytes = u.data.clone();

            let mut utf8_bytes: Vec<Vec<u8>> = Vec::new();

            for bytes in scalar_bytes {
                // Reconstruct the scalar value assuming big-endian byte order
                let mut scalar_value: u32 = 0;
                let num_bytes = bytes.len();
                for (i, &byte) in bytes.iter().enumerate() {
                    scalar_value |= (byte as u32) << (8 * (num_bytes - 1 - i));
                }
                println!("Scalar value: {scalar_value}");
                // Convert the scalar value to a char if it's a valid Unicode scalar value
                if let Some(character) = std::char::from_u32(scalar_value) {
                    if character.is_ascii() {
                        utf8_bytes.push(vec![character as u8]);
                    } else {
                        let mut buf = [0; 4];
                        let encoded_str = character.encode_utf8(&mut buf);
                        utf8_bytes.push(encoded_str.as_bytes().to_vec());
                    }
                }
            }

            println!("Output UTF-8 Data: {:?}", utf8_bytes);

            Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data {
                data: utf8_bytes,
            })))
        }
        _ => input,
    }
}

pub fn unicode_scalars_from_string(str: String) -> Value {
    let validated_utf8_str = std::str::from_utf8(str.as_bytes()).unwrap();
    let mut data = vec![];

    for char in validated_utf8_str.chars() {
        let scalar_value = char as u32; 
        let mut scalar_bytes = scalar_value.to_be_bytes().to_vec();
        scalar_bytes.retain(|&x| x != 0); 

        data.push(scalar_bytes);
    }

    
    Value::Sequence(SequenceData::String(CharType::UTF8(UTF8Data { data })))
}

#[allow(clippy::result_unit_err)]
pub fn parse_utf8(input: Result<Option<Value>, ()>) -> Result<Option<Value>, ()> {
    if let Ok(ok_value) = input {
        if let Some(v) = ok_value {
            Ok(Some(unicode_to_byte_sequence(v)))
        } else {
            Ok(None)
        }
    } else {
        input
    }
}

pub fn crosscheck(snippet: &str, expected: Result<Option<Value>, ()>) {
    let compiled = evaluate_at(snippet, StacksEpochId::latest(), ClarityVersion::latest());
    let interpreted = execute(snippet);

    assert_eq!(
        compiled.as_ref().map_err(|_| &()),
        interpreted.as_ref().map_err(|_| &()),
        "Compiled and interpreted results diverge!\ncompiled: {:?}\ninterpreted: {:?}",
        &compiled,
        &interpreted
    );

    let parsed_expected = parse_utf8(expected.clone());

    println!("Expected Original:{:?}", expected);
    println!("Expected Parsed:{:?}", parsed_expected);

    assert_eq!(
        compiled.map_err(|_| ()),
        parsed_expected,
        // expected,
        "Not the expected result"
    );
    println!("-------------------------------------");
    println!("-------------------------------------");
}

#[test]
fn test_evaluate_snippet() {
    assert_eq!(evaluate("(+ 1 2)"), Ok(Some(Value::Int(3))));
}
