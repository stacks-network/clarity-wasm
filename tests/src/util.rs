#![allow(dead_code)]

use clar2wasm::compile;
use clarity::types::StacksEpochId;
use clarity::vm::{
    analysis::ContractAnalysis,
    costs::LimitedCostTracker,
    database::MemoryBackingStore,
    types::{FunctionType, QualifiedContractIdentifier, StandardPrincipalData, TypeSignature},
    ClarityVersion, ContractName,
};
use wasmtime::{AsContextMut, Engine, FuncType, Instance, Linker, Module, Store, Val, ValType};

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

/// A simple wrapper for WASMTime to help reduce the amount of boilerplate needed
/// in test code. The wrapper compiles the specified contract using `clar2wasm` and
/// stores a copy of its contract analysis for type inferrence when calling functions.
pub struct WasmtimeHelper {
    module: Module,
    instance: Instance,
    store: Box<Store<()>>,
    contract_analysis: ContractAnalysis,
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

        let wasm = compile_result.module.emit_wasm();
        let contract_analysis = compile_result.contract_analysis;

        let engine = Engine::default();
        let module = Module::from_binary(&engine, wasm.as_slice()).unwrap();
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);

        let instance = linker.instantiate(store.as_context_mut(), &module).unwrap();

        WasmtimeHelper {
            module,
            instance,
            store: Box::new(store),
            contract_analysis,
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

    /// Calls the specified public Clarity function in the generated contract WASM binary.
    pub fn call_public_function(&mut self, name: &str, params: &[Val]) -> ClarityWasmResult {
        let fn_type = self
            .contract_analysis
            .get_public_function_type(name)
            .expect("Function not found");

        eprintln!("Clarity function type: {:?}", &fn_type);

        let func_type = Self::generate_wasmtime_func_signature(fn_type);

        let func = self
            .instance
            .get_func(self.store.as_context_mut(), name)
            .expect("Provided function name was not found in the generated WASM binary.");

        let mut results = vec![Val::I32(0); func_type.results().len()];
        eprintln!("Results count: {}", results.len());

        func.call(self.store.as_context_mut(), params, &mut results)
            .unwrap();

        eprint!("Params: {:?}, Results: {:?}", params, results);

        Self::map_wasm_result(fn_type, &results)
    }

    /// Experimental
    pub fn eval(&mut self) -> Vec<ClarityWasmResult> {
        /*let func = self
            .instance
            .get_func(self.store.as_context_mut(), ".top-level")
            .expect("Default (top-level) function not found.");

        let func_type = func.ty(self.store.as_context_mut());*/

        for expr in self.contract_analysis.expressions.iter() {
            let expr_type = self
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
