#![allow(dead_code)]

use std::borrow::BorrowMut;
use std::collections::HashMap;

use clar2wasm::compile;
use clarity::consts::CHAIN_ID_TESTNET;
use clarity::types::StacksEpochId;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::database::ClarityDatabase;
use clarity::vm::types::{BufferLength, SequenceSubtype, StringSubtype};
use clarity::vm::{
    analysis::ContractAnalysis,
    costs::LimitedCostTracker,
    database::MemoryBackingStore,
    types::{FunctionType, QualifiedContractIdentifier, StandardPrincipalData, TypeSignature},
    ClarityVersion, ContractName,
};
use clarity::vm::{CallStack, ContractContext, Environment, LocalContext, Value};
use wasmtime::{
    AsContextMut, Caller, Engine, FuncType, Instance, Linker, Module, Store, Val, ValType,
};

use crate::datastore::{BurnDatastore, Datastore, StacksConstants};

#[derive(Debug, PartialEq)]
pub enum ClarityWasmResult {
    Int {
        high: i64,
        low: i64,
    },
    UInt {
        high: i64,
        low: i64,
    },
    Bool {
        value: i32,
    },
    Principal {
        pointer: i32,
    },
    Buff {
        pointer: i32,
        length: i32,
    },
    StringAscii {
        pointer: i32,
        length: i32,
    },
    StringUtf8 {
        pointer: i32,
        length: i32,
    },
    List {
        pointer: i32,
        length: i32,
    },
    Tuple {
        values: Vec<Self>,
    },
    Optional {
        indicator: i32,
        value: Option<Box<Self>>,
    },
    Response {
        indicator: i32,
        ok_value: Option<Box<Self>>,
        err_value: Option<Box<Self>>,
    },
    NoType,
}

pub struct ClarityWasmContext {
    /// Contract context for runtime execution
    pub contract_context: ContractContext,
    /// The contract analysis for the compiled contract
    pub contract_analysis: ContractAnalysis,
    /// Map an identifier from a contract to an integer id for simple access
    pub identifier_map: HashMap<i32, String>,
}

impl ClarityWasmContext {
    pub fn new(contract_context: ContractContext, contract_analysis: ContractAnalysis) -> Self {
        ClarityWasmContext {
            contract_analysis,
            contract_context,
            identifier_map: HashMap::new(),
        }
    }
}

/// A simple wrapper for WASMTime to help reduce the amount of boilerplate needed
/// in test code. The wrapper compiles the specified contract using `clar2wasm` and
/// stores a copy of its contract analysis for type inferrence when calling functions.
pub struct WasmtimeHelper {
    module: Module,
    instance: Instance,
    store: Box<Store<ClarityWasmContext>>,
}

impl WasmtimeHelper {
    pub fn new(contract_name: &str) -> Self {
        let contract_str =
            std::fs::read_to_string(format!("contracts/{contract_name}.clar")).unwrap();

        let contract_id = QualifiedContractIdentifier::new(
            StandardPrincipalData::transient(),
            ContractName::from(contract_name),
        );
        let cost_tracker = LimitedCostTracker::Free;
        let mut clarity_store = MemoryBackingStore::new();

        let mut compile_result = compile(
            contract_str.as_str(),
            &contract_id,
            cost_tracker,
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch24,
            &mut clarity_store,
        )
        .expect("Failed to compile contract.");

        let mut datastore = Datastore::new();
        let constants = StacksConstants {
            burn_start_height: 0,
            pox_prepare_length: 0,
            pox_reward_cycle_length: 0,
            pox_rejection_fraction: 0,
            epoch_21_start_height: 0,
        };
        let burn_datastore = BurnDatastore::new(constants);
        let mut conn = ClarityDatabase::new(&mut datastore, &burn_datastore, &burn_datastore);
        conn.begin();
        conn.set_clarity_epoch_version(StacksEpochId::Epoch24);
        conn.commit();
        let cost_tracker = LimitedCostTracker::new_free();
        let mut global_context = GlobalContext::new(
            false,
            CHAIN_ID_TESTNET,
            conn,
            cost_tracker,
            StacksEpochId::Epoch24,
        );

        let mut contract_context = ContractContext::new(contract_id.clone(), ClarityVersion::Clarity2);
        let context = LocalContext::new();
        let mut call_stack = CallStack::new();
        let mut env = Environment::new(
            &mut global_context,
            &mut contract_context,
            &mut call_stack,
            None,
            None,
            None,
        );

        let wasm = compile_result.module.emit_wasm();
        let contract_analysis = compile_result.contract_analysis;
        let context = ClarityWasmContext::new(contract_context, contract_analysis);

        let engine = Engine::default();
        let module = Module::from_binary(&engine, wasm.as_slice()).unwrap();
        let mut store = Store::new(&engine, context);
        let mut linker = Linker::new(&engine);

        // Link in the host interface functions.
        linker
            .func_wrap(
                "clarity",
                "define_variable",
                |mut caller: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 name_offset: i32,
                 name_length: i32,
                 value_offset: i32,
                 value_length: i32| {
                    // Read the variable name string from the memory
                    let name = WasmtimeHelper::read_identifier_from_wasm(
                        &mut caller,
                        name_offset,
                        name_length,
                    );

                    // Read the initial value from the memory
                    let ty = caller
                        .data()
                        .contract_analysis
                        .get_persisted_variable_type(name.as_str())
                        .expect("failed to get variable type")
                        .clone();
                    let value = WasmtimeHelper::read_from_wasm(
                        &mut caller,
                        &ty,
                        value_offset,
                        value_length,
                    );

                    // Store the mapping of variable name to identifier
                    caller
                        .data_mut()
                        .identifier_map
                        .insert(identifier, name.clone());

                    // TODO: Call into the contract context to create the variable
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "clarity",
                "get_variable",
                |caller: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 _return_offset: i32,
                 _return_length: i32| {
                    let var_name = caller
                        .data()
                        .identifier_map
                        .get(&identifier)
                        .expect("failed to get variable name");
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "clarity",
                "set_variable",
                |_: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 _return_offset: i32,
                 _return_length: i32| {
                    println!("var-set: {identifier}");
                },
            )
            .unwrap();

        // Create a log function for debugging.
        linker
            .func_wrap(
                "",
                "log",
                |_: Caller<'_, ClarityWasmContext>, param: i64| {
                    println!("log: {param}");
                },
            )
            .unwrap();

        let instance = linker.instantiate(store.as_context_mut(), &module).unwrap();

        let mut helper = WasmtimeHelper {
            module,
            instance,
            store: Box::new(store),
        };

        // Run the top-level expressions
        helper.call_top_level();

        helper
    }

    /// Read an identifier (string) from the WASM memory at `offset` with `length`.
    fn read_identifier_from_wasm(
        caller: &mut Caller<'_, ClarityWasmContext>,
        offset: i32,
        length: i32,
    ) -> String {
        // Get the memory from the caller
        let memory = caller
            .get_export("memory")
            .and_then(|export| export.into_memory())
            .expect("instance memory export");

        let mut buffer: Vec<u8> = vec![0; length as usize];
        memory
            .read(caller, offset as usize, &mut buffer)
            .expect("failed to read variable name");
        String::from_utf8(buffer).expect("failed to convert memory contents to string")
    }

    /// Read a value from the WASM memory at `offset` with `length` given the provided
    /// Clarity `TypeSignature`.
    fn read_from_wasm(
        caller: &mut Caller<'_, ClarityWasmContext>,
        ty: &TypeSignature,
        offset: i32,
        length: i32,
    ) -> Value {
        // Get the memory from the caller
        let memory = caller
            .get_export("memory")
            .and_then(|export| export.into_memory())
            .expect("instance memory export");

        match ty {
            TypeSignature::UIntType => {
                assert!(
                    length == 16,
                    "expected uint length to be 16 bytes, found {length}"
                );
                let mut buffer: [u8; 8] = [0; 8];
                memory
                    .read(caller.borrow_mut(), offset as usize, &mut buffer)
                    .expect("failed to read int");
                let high = u64::from_le_bytes(buffer) as u128;
                memory
                    .read(caller.borrow_mut(), (offset + 8) as usize, &mut buffer)
                    .expect("failed to read int");
                let low = u64::from_le_bytes(buffer) as u128;
                Value::UInt((high << 64) | low)
            }
            TypeSignature::IntType => {
                assert!(
                    length == 16,
                    "expected int length to be 16 bytes, found {length}"
                );
                let mut buffer: [u8; 8] = [0; 8];
                memory
                    .read(caller.borrow_mut(), offset as usize, &mut buffer)
                    .expect("failed to read int");
                let high = u64::from_le_bytes(buffer) as u128;
                memory
                    .read(caller.borrow_mut(), (offset + 8) as usize, &mut buffer)
                    .expect("failed to read int");
                let low = u64::from_le_bytes(buffer) as u128;
                Value::Int(((high << 64) | low) as i128)
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                type_length,
            ))) => {
                assert!(
                    type_length
                        >= &BufferLength::try_from(length as u32).expect("invalid buffer length"),
                    "expected string length to be less than the type length"
                );
                let mut buffer: Vec<u8> = vec![0; length as usize];
                memory
                    .read(caller, offset as usize, &mut buffer)
                    .expect("failed to read variable name");
                Value::string_ascii_from_bytes(buffer)
                    .expect("failed to convert memory contents to string")
            }
            _ => panic!("unsupported type"),
        }
    }

    /// Generates a WASMTime function signature (both input and return arguments), provided the
    /// given Clarity `FunctionType`.
    fn generate_wasmtime_func_signature(fn_sig: &FunctionType) -> FuncType {
        let mut params = Vec::<ValType>::new();
        let mut returns = Vec::<ValType>::new();

        match fn_sig {
            FunctionType::Fixed(func) => {
                for arg in func.args.iter() {
                    let mut arg_sig = Self::get_wasmtime_arg(&arg.signature);
                    params.append(&mut arg_sig);
                }

                let mut returns_sig = Self::get_wasmtime_arg(&func.returns);
                returns.append(&mut returns_sig);
            }
            _ => panic!("Not implemented"),
        }

        let func_type = FuncType::new(params, returns);
        eprintln!("Wasmtime FuncType: {:?}", func_type);
        func_type
    }

    /// Creates the type signature expected by WASMTime for the provided Clarity `TypeSignature`.
    fn get_wasmtime_arg(type_sig: &TypeSignature) -> Vec<ValType> {
        match type_sig {
            TypeSignature::IntType | TypeSignature::UIntType => vec![ValType::I64, ValType::I64],
            TypeSignature::BoolType => vec![ValType::I32],
            TypeSignature::SequenceType(_) => vec![ValType::I32, ValType::I32],
            TypeSignature::ResponseType(resp) => {
                let mut sig = vec![ValType::I32; 1];
                let mut ok_type = Self::get_wasmtime_arg(&resp.0);
                let mut err_type = Self::get_wasmtime_arg(&resp.1);
                sig.append(&mut ok_type);
                sig.append(&mut err_type);
                sig
            }
            TypeSignature::NoType => vec![ValType::I32],
            _ => panic!("Not implemented"),
        }
    }

    /// Maps the result from a WASM function call given the provided Clarity `FunctionType`.
    fn map_wasm_result(fn_sig: &FunctionType, result: &[Val]) -> ClarityWasmResult {
        match fn_sig {
            FunctionType::Fixed(func) => {
                let (result, _) = Self::map_wasm_value(&func.returns, 0, result);
                result
            }
            _ => panic!("Function type '{:?}' not implemented.", &fn_sig),
        }
    }

    /// Maps an individual value in a WASM function call result.
    fn map_wasm_value(
        type_sig: &TypeSignature,
        index: usize,
        buffer: &[Val],
    ) -> (ClarityWasmResult, usize) {
        match type_sig {
            TypeSignature::IntType => {
                let upper = buffer[index].unwrap_i64();
                let lower = buffer[index + 1].unwrap_i64();
                (
                    ClarityWasmResult::Int {
                        high: upper,
                        low: lower,
                    },
                    2,
                )
            }
            TypeSignature::UIntType => {
                let upper = buffer[index].unwrap_i64();
                let lower = buffer[index + 1].unwrap_i64();
                (
                    ClarityWasmResult::UInt {
                        high: upper,
                        low: lower,
                    },
                    2,
                )
            }
            TypeSignature::NoType => (ClarityWasmResult::NoType, 1),
            TypeSignature::ResponseType(response) => {
                let (ok, increment_ok) = Self::map_wasm_value(&response.0, index + 1, buffer);
                let (err, increment_err) =
                    Self::map_wasm_value(&response.1, index + 1 + increment_ok, buffer);
                (
                    ClarityWasmResult::Response {
                        indicator: buffer[index].unwrap_i32(),
                        ok_value: if ok == ClarityWasmResult::NoType {
                            None
                        } else {
                            Some(Box::new(ok))
                        },
                        err_value: if err == ClarityWasmResult::NoType {
                            None
                        } else {
                            Some(Box::new(err))
                        },
                    },
                    index + 1 + increment_ok + increment_err,
                )
            }
            _ => panic!("WASM value type not implemented: {:?}", type_sig),
        }
    }

    /// Runs the top-level expressions in a clarity contract, by calling the
    /// `.top-level` function.
    pub fn call_top_level(&mut self) {
        let func = self
            .instance
            .get_func(self.store.as_context_mut(), ".top-level")
            .expect(".top-level function was not found in the generated WASM binary.");
        let mut results = [];

        func.call(self.store.as_context_mut(), &[], &mut results)
            .unwrap();
    }

    /// Calls the specified public Clarity function in the generated contract WASM binary.
    pub fn call_public_function(&mut self, name: &str, params: &[Val]) -> ClarityWasmResult {
        let fn_type = self
            .store
            .data()
            .contract_analysis
            .get_public_function_type(name)
            .expect("Function not found")
            .clone();

        eprintln!("Clarity function type: {:?}", &fn_type);

        let func_type = Self::generate_wasmtime_func_signature(&fn_type);

        let func = self
            .instance
            .get_func(self.store.as_context_mut(), name)
            .expect("Provided function name was not found in the generated WASM binary.");

        let mut results = vec![Val::I32(0); func_type.results().len()];
        eprintln!("Results count: {}", results.len());

        func.call(self.store.as_context_mut(), params, &mut results)
            .unwrap();

        eprint!("Params: {:?}, Results: {:?}", params, results);

        Self::map_wasm_result(&fn_type, &results)
    }

    /// Experimental
    pub fn eval(&mut self) -> Vec<ClarityWasmResult> {
        /*let func = self
            .instance
            .get_func(self.store.as_context_mut(), ".top-level")
            .expect("Default (top-level) function not found.");

        let func_type = func.ty(self.store.as_context_mut());*/

        for expr in self.store.data().contract_analysis.expressions.iter() {
            let expr_type = self
                .store
                .data()
                .contract_analysis
                .type_map
                .as_ref()
                .expect("Type analysis must be run")
                .get_type(expr)
                .unwrap();
            eprintln!("> Expr: {:?}", expr);
            eprintln!("Expression type: {:?}", expr_type);

            /*** FOR FUTURE REFERENCE (cylwit)
            let result_signature = Self::get_wasmtime_arg(expr_type);
            let mut results = Vec::<Val>::with_capacity(result_signature.len());
            for x in result_signature.iter() {
                match x {
                    ValType::I32 => results.push(Val::I32(0)),
                    ValType::I64 => results.push(Val::I64(0)),
                    _ => unimplemented!()
                }
            }

            func.call(self.store.as_context_mut(), &[], &mut results)
                .expect("WASM function call failed.");

            eprintln!("Results: {:?}", results);*/
        }

        panic!("Not implemented");
    }
}
