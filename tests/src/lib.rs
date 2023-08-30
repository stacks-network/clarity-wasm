#![allow(dead_code)]

pub mod datastore;

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

// /// A simple wrapper for WASMTime to help reduce the amount of boilerplate needed
// /// in test code. The wrapper compiles the specified contract using `clar2wasm` and
// /// stores a copy of its contract analysis for type inferrence when calling functions.
// pub struct WasmtimeHelper<'a, 'b, 'hooks> {
//     module: Module,
//     instance: Instance,
//     store: Box<Store<ClarityWasmContext<'a, 'b, 'hooks>>>,
// }

// /// Generates a WASMTime function signature (both input and return arguments), provided the
// /// given Clarity `FunctionType`.
// fn generate_wasmtime_func_signature(fn_sig: &FunctionType) -> FuncType {
//     let mut params = Vec::<ValType>::new();
//     let mut returns = Vec::<ValType>::new();

//     match fn_sig {
//         FunctionType::Fixed(func) => {
//             for arg in func.args.iter() {
//                 let mut arg_sig = get_wasmtime_arg(&arg.signature);
//                 params.append(&mut arg_sig);
//             }

//             let mut returns_sig = get_wasmtime_arg(&func.returns);
//             returns.append(&mut returns_sig);
//         }
//         _ => panic!("Not implemented"),
//     }

//     FuncType::new(params, returns)
// }

// /// Creates the type signature expected by WASMTime for the provided Clarity `TypeSignature`.
// fn get_wasmtime_arg(type_sig: &TypeSignature) -> Vec<ValType> {
//     match type_sig {
//         TypeSignature::IntType | TypeSignature::UIntType => vec![ValType::I64, ValType::I64],
//         TypeSignature::BoolType => vec![ValType::I32],
//         TypeSignature::SequenceType(_) => vec![ValType::I32, ValType::I32],
//         TypeSignature::ResponseType(resp) => {
//             let mut sig = vec![ValType::I32; 1];
//             let mut ok_type = get_wasmtime_arg(&resp.0);
//             let mut err_type = get_wasmtime_arg(&resp.1);
//             sig.append(&mut ok_type);
//             sig.append(&mut err_type);
//             sig
//         }
//         TypeSignature::NoType => vec![ValType::I32],
//         _ => panic!("Not implemented"),
//     }
// }

// /// Maps the result from a WASM function call given the provided Clarity `FunctionType`.
// fn map_wasm_result(fn_sig: &FunctionType, result: &[Val]) -> Value {
//     match fn_sig {
//         FunctionType::Fixed(func) => {
//             let (result, _) = map_wasm_value(&func.returns, 0, result);
//             result
//         }
//         _ => panic!("Function type '{:?}' not implemented.", &fn_sig),
//     }
// }

// /// Maps an individual value in a WASM function call result.
// fn map_wasm_value(type_sig: &TypeSignature, index: usize, buffer: &[Val]) -> (Value, usize) {
//     match type_sig {
//         TypeSignature::IntType => {
//             let upper = buffer[index].unwrap_i64();
//             let lower = buffer[index + 1].unwrap_i64();
//             (Value::Int(((upper as i128) << 64) | lower as i128), 2)
//         }
//         TypeSignature::UIntType => {
//             let upper = buffer[index].unwrap_i64();
//             let lower = buffer[index + 1].unwrap_i64();
//             (Value::UInt(((upper as u128) << 64) | lower as u128), 2)
//         }
//         TypeSignature::BoolType => (Value::Bool(buffer[index].unwrap_i32() != 0), 1),
//         TypeSignature::OptionalType(optional) => {
//             let (value, increment) = map_wasm_value(optional, index + 1, buffer);
//             (
//                 if buffer[index].unwrap_i32() == 1 {
//                     Value::some(value).unwrap()
//                 } else {
//                     Value::none()
//                 },
//                 increment + 1,
//             )
//         }
//         TypeSignature::ResponseType(response) => {
//             let (ok, increment_ok) = map_wasm_value(&response.0, index + 1, buffer);
//             let (err, increment_err) =
//                 map_wasm_value(&response.1, index + 1 + increment_ok, buffer);
//             (
//                 if buffer[index].unwrap_i32() == 1 {
//                     Value::okay(ok).unwrap()
//                 } else {
//                     Value::error(err).unwrap()
//                 },
//                 index + 1 + increment_ok + increment_err,
//             )
//         }
//         // A `NoType` will be a dummy value that should not be used.
//         TypeSignature::NoType => (Value::none(), 1),
//         _ => panic!("WASM value type not implemented: {:?}", type_sig),
//     }
// }

// impl<'a, 'b, 'hooks> WasmtimeHelper<'a, 'b, 'hooks> {
//     /// Creates a new `WasmtimeHelper` using the Clarity in the provided string reference.
//     pub fn new_from_str(
//         contract_id: QualifiedContractIdentifier,
//         global_context: &'b mut GlobalContext<'a, 'hooks>,
//         contract_context: &'b mut ContractContext,
//         contract_text: &str,
//     ) -> Self {
//         let cost_tracker = LimitedCostTracker::Free;
//         let mut clarity_store = MemoryBackingStore::new();

//         let mut compile_result = compile(
//             contract_text,
//             &contract_id,
//             cost_tracker,
//             ClarityVersion::Clarity2,
//             StacksEpochId::Epoch24,
//             &mut clarity_store,
//         )
//         .expect("Failed to compile contract.");

//         let wasm = compile_result.module.emit_wasm();
//         let contract_analysis = compile_result.contract_analysis;
//         let context = ClarityWasmContext::new(global_context, contract_context, contract_analysis);

//         let engine = Engine::default();
//         let module = Module::from_binary(&engine, wasm.as_slice()).unwrap();
//         let mut store = Store::new(&engine, context);
//         let mut linker = Linker::new(&engine);

//         // Link in the host interface functions.
//         Self::link_define_variable_fn(&mut linker);
//         Self::link_get_variable_fn(&mut linker);
//         Self::link_set_variable_fn(&mut linker);

//         // Create a log function for debugging.
//         linker
//             .func_wrap(
//                 "",
//                 "log",
//                 |_: Caller<'_, ClarityWasmContext>, param: i64| {
//                     println!("log: {param}");
//                 },
//             )
//             .unwrap();

//         let instance = linker.instantiate(store.as_context_mut(), &module).unwrap();

//         let mut helper = WasmtimeHelper {
//             module,
//             instance,
//             store: Box::new(store),
//         };

//         // Run the top-level expressions
//         helper.call_top_level();

//         helper
//     }

//     /// Creates a new `WasmtimeHelper` using Clarity in the file at the specified path.
//     pub fn new_from_file(
//         contract_id: QualifiedContractIdentifier,
//         global_context: &'b mut GlobalContext<'a, 'hooks>,
//         contract_context: &'b mut ContractContext,
//     ) -> Self {
//         let contract_str =
//             std::fs::read_to_string(format!("contracts/{}.clar", contract_id.name)).unwrap();

//         Self::new_from_str(
//             contract_id,
//             global_context,
//             contract_context,
//             contract_str.as_str(),
//         )
//     }

//     /// Calls the specified public Clarity function in the generated contract WASM binary.
//     pub fn call_public_function(&mut self, name: &str, params: &[Val]) -> Value {
//         let fn_type = self
//             .store
//             .data()
//             .contract_analysis
//             .get_public_function_type(name)
//             .expect("Function not found")
//             .clone();

//         let func_type = generate_wasmtime_func_signature(&fn_type);

//         let func = self
//             .instance
//             .get_func(self.store.as_context_mut(), name)
//             .expect("Provided function name was not found in the generated WASM binary.");

//         let mut results = vec![Val::I32(0); func_type.results().len()];

//         func.call(self.store.as_context_mut(), params, &mut results)
//             .unwrap();

//         map_wasm_result(&fn_type, &results)
//     }

//     /// Experimental
//     pub fn eval(&mut self) -> Vec<Value> {
//         /*let func = self
//             .instance
//             .get_func(self.store.as_context_mut(), ".top-level")
//             .expect("Default (top-level) function not found.");

//         let func_type = func.ty(self.store.as_context_mut());*/

//         for expr in self.store.data().contract_analysis.expressions.iter() {
//             let expr_type = self
//                 .store
//                 .data()
//                 .contract_analysis
//                 .type_map
//                 .as_ref()
//                 .expect("Type analysis must be run")
//                 .get_type(expr)
//                 .unwrap();
//             eprintln!("> Expr: {:?}", expr);
//             eprintln!("Expression type: {:?}", expr_type);

//             /*** FOR FUTURE REFERENCE (cylwit)
//             let result_signature = get_wasmtime_arg(expr_type);
//             let mut results = Vec::<Val>::with_capacity(result_signature.len());
//             for x in result_signature.iter() {
//                 match x {
//                     ValType::I32 => results.push(Val::I32(0)),
//                     ValType::I64 => results.push(Val::I64(0)),
//                     _ => unimplemented!()
//                 }
//             }

//             func.call(self.store.as_context_mut(), &[], &mut results)
//                 .expect("WASM function call failed.");

//             eprintln!("Results: {:?}", results);*/
//         }

//         panic!("Not implemented");
//     }
// }
