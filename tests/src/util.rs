#![allow(dead_code)]

use std::borrow::BorrowMut;
use std::collections::HashMap;

use clar2wasm::compile;
use clarity::types::StacksEpochId;
use clarity::vm::contexts::GlobalContext;
use clarity::vm::types::{BufferLength, SequenceSubtype, StringSubtype};
use clarity::vm::{
    analysis::ContractAnalysis,
    costs::LimitedCostTracker,
    database::MemoryBackingStore,
    types::{FunctionType, QualifiedContractIdentifier, TypeSignature},
    ClarityVersion,
};
use clarity::vm::{ClarityName, ContractContext, Value};
use wasmtime::{
    AsContextMut, Caller, Engine, FuncType, Instance, Linker, Module, Store, Val, ValType,
};

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

pub struct ClarityWasmContext<'a, 'b, 'hooks> {
    /// The global context in which to execute.
    pub global_context: &'b mut GlobalContext<'a, 'hooks>,
    /// Context for this contract. This will be filled in when running the
    /// top-level expressions, then used when calling functions.
    pub contract_context: &'b mut ContractContext,
    /// The contract analysis for the compiled contract
    pub contract_analysis: ContractAnalysis,
    /// Map an identifier from a contract to an integer id for simple access
    pub identifier_map: HashMap<i32, String>,
}

impl<'a, 'b, 'hooks> ClarityWasmContext<'a, 'b, 'hooks> {
    pub fn new(
        global_context: &'b mut GlobalContext<'a, 'hooks>,
        contract_context: &'b mut ContractContext,
        contract_analysis: ContractAnalysis,
    ) -> Self {
        ClarityWasmContext {
            global_context,
            contract_analysis,
            contract_context,
            identifier_map: HashMap::new(),
        }
    }
}

/// A simple wrapper for WASMTime to help reduce the amount of boilerplate needed
/// in test code. The wrapper compiles the specified contract using `clar2wasm` and
/// stores a copy of its contract analysis for type inferrence when calling functions.
pub struct WasmtimeHelper<'a, 'b, 'hooks> {
    module: Module,
    instance: Instance,
    store: Box<Store<ClarityWasmContext<'a, 'b, 'hooks>>>,
}

/// Generates a WASMTime function signature (both input and return arguments), provided the
/// given Clarity `FunctionType`.
fn generate_wasmtime_func_signature(fn_sig: &FunctionType) -> FuncType {
    let mut params = Vec::<ValType>::new();
    let mut returns = Vec::<ValType>::new();

    match fn_sig {
        FunctionType::Fixed(func) => {
            for arg in func.args.iter() {
                let mut arg_sig = get_wasmtime_arg(&arg.signature);
                params.append(&mut arg_sig);
            }

            let mut returns_sig = get_wasmtime_arg(&func.returns);
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
            let mut ok_type = get_wasmtime_arg(&resp.0);
            let mut err_type = get_wasmtime_arg(&resp.1);
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
            let (result, _) = map_wasm_value(&func.returns, 0, result);
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
            let (ok, increment_ok) = map_wasm_value(&response.0, index + 1, buffer);
            let (err, increment_err) =
                map_wasm_value(&response.1, index + 1 + increment_ok, buffer);
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

impl<'a, 'b, 'hooks> WasmtimeHelper<'a, 'b, 'hooks> {
    pub fn new(
        contract_id: QualifiedContractIdentifier,
        global_context: &'b mut GlobalContext<'a, 'hooks>,
        contract_context: &'b mut ContractContext,
    ) -> Self {
        let contract_str =
            std::fs::read_to_string(format!("contracts/{}.clar", contract_id.name)).unwrap();

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

        let wasm = compile_result.module.emit_wasm();
        let contract_analysis = compile_result.contract_analysis;
        let context = ClarityWasmContext::new(global_context, contract_context, contract_analysis);

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
                    let contract = caller.data().contract_context.contract_identifier.clone();
                    let epoch = caller.data().global_context.epoch_id;
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

                    // Create the variable in the global context
                    let data_type = caller.data_mut().global_context.database.create_variable(
                        &contract,
                        name.as_str(),
                        ty,
                    );

                    // Store the variable in the global context
                    caller
                        .data_mut()
                        .global_context
                        .database
                        .set_variable(&contract, name.as_str(), value, &data_type, &epoch)
                        .unwrap();

                    caller
                        .data_mut()
                        .contract_context
                        .meta_data_var
                        .insert(ClarityName::from(name.as_str()), data_type.clone());
                },
            )
            .unwrap();

        linker
            .func_wrap(
                "clarity",
                "get_variable",
                |mut caller: Caller<'_, ClarityWasmContext>,
                 identifier: i32,
                 return_offset: i32,
                 return_length: i32| {
                    let var_name = caller
                        .data()
                        .identifier_map
                        .get(&identifier)
                        .expect("failed to get variable name")
                        .clone();
                    let contract = caller.data().contract_context.contract_identifier.clone();
                    let data_types = caller
                        .data()
                        .contract_context
                        .meta_data_var
                        .get(var_name.as_str())
                        .unwrap()
                        .clone(); // FIXME
                    let value = caller
                        .data_mut()
                        .global_context
                        .database
                        .lookup_variable_with_size(
                            &contract,
                            var_name.as_str(),
                            &data_types,
                            &StacksEpochId::Epoch24,
                        )
                        .unwrap()
                        .value;

                    WasmtimeHelper::write_to_wasm(
                        &mut caller,
                        &data_types.value_type,
                        return_offset,
                        return_length,
                        value,
                    );
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

    /// Write a value to the Wasm memory at `offset` with `length` given the
    /// provided Clarity `TypeSignature`.'
    fn write_to_wasm(
        caller: &mut Caller<'_, ClarityWasmContext>,
        ty: &TypeSignature,
        offset: i32,
        length: i32,
        value: Value,
    ) {
        let memory = caller
            .get_export("memory")
            .and_then(|export| export.into_memory())
            .expect("instance memory export");

        match ty {
            TypeSignature::IntType => {
                assert!(
                    length == 16,
                    "expected int length to be 16 bytes, found {length}"
                );
                let mut buffer: [u8; 8] = [0; 8];
                let i = value.expect_i128();
                let high = (i >> 64) as u64;
                let low = (i & 0xffff_ffff_ffff_ffff) as u64;
                buffer.copy_from_slice(&high.to_le_bytes());
                memory
                    .write(caller.borrow_mut(), offset as usize, &buffer)
                    .expect("failed to write int");
                buffer.copy_from_slice(&low.to_le_bytes());
                memory
                    .write(caller.borrow_mut(), (offset + 8) as usize, &buffer)
                    .expect("failed to write int");
            }
            _ => panic!("unsupported type"),
        };
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

        let func_type = generate_wasmtime_func_signature(&fn_type);

        let func = self
            .instance
            .get_func(self.store.as_context_mut(), name)
            .expect("Provided function name was not found in the generated WASM binary.");

        let mut results = vec![Val::I32(0); func_type.results().len()];
        eprintln!("Results count: {}", results.len());

        func.call(self.store.as_context_mut(), params, &mut results)
            .unwrap();

        eprint!("Params: {:?}, Results: {:?}", params, results);

        map_wasm_result(&fn_type, &results)
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
            let result_signature = get_wasmtime_arg(expr_type);
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
