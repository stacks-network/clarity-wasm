use clarity::vm::{
    analysis::ContractAnalysis, diagnostic::DiagnosableError, functions::NativeFunctions,
    SymbolicExpression,
};
use walrus::{FunctionBuilder, Module, ModuleConfig, ValType, ir::BinaryOp};

use crate::ast_visitor::{traverse, ASTVisitor};

pub struct WasmGenerator {
    module: Module,
    error: Option<GeneratorError>,
    /// Top-level code in the contract. When we are in the top-level context,
    /// this will be None, and the current context will be the top-level.
    top_level_builder: Option<FunctionBuilder>,
    /// Current function context we are in.
    current_function: FunctionBuilder,
}

pub enum GeneratorError {
    NotImplemented,
    InternalError(String),
}

impl DiagnosableError for GeneratorError {
    fn message(&self) -> String {
        match self {
            GeneratorError::NotImplemented => "Not implemented".to_string(),
            GeneratorError::InternalError(msg) => format!("Internal error: {}", msg),
        }
    }

    fn suggestion(&self) -> Option<String> {
        None
    }
}

impl WasmGenerator {
    pub fn new() -> WasmGenerator {
        let mut module = Module::with_config(ModuleConfig::default());
        let top_level = FunctionBuilder::new(&mut module.types, &[], &[ValType::I64]);

        WasmGenerator {
            module,
            error: None,
            top_level_builder: None,
            current_function: top_level,
        }
    }

    pub fn generate(
        mut self,
        contract_analysis: ContractAnalysis,
    ) -> Result<Vec<u8>, GeneratorError> {
        // println!("{:?}", contract_analysis.expressions);
        traverse(&mut self, &contract_analysis.expressions);

        if let Some(err) = self.error {
            return Err(err);
        }

        // Insert a return instruction at the end of the top-level function
        self.current_function.func_body().return_();

        self.module.exports.add(".top-level", self.current_function.finish(vec![], &mut self.module.funcs));

        // TODO: Remove this - for debugging only
        self.module.emit_wasm_file("out.wasm").unwrap();

        Ok(self.module.emit_wasm())
    }
}

impl<'a> ASTVisitor<'a> for WasmGenerator {
    fn visit_arithmetic(
        &mut self,
        _expr: &'a SymbolicExpression,
        func: NativeFunctions,
        _operands: &'a [SymbolicExpression],
    ) -> bool {
        match func {
            NativeFunctions::Add => {
                // TODO: Handle 128-bit

                // TODO: Handle > 2 operands
                // e.g. (+ 1 2 3) should become:
                // i64.const 1
                // i64.const 2
                // i64.add
                // i64.const 3
                // i64.add
                self.current_function.func_body().binop(BinaryOp::I64Add);
                true
            }
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                false
            }
        }
    }

    fn visit_literal_value(&mut self, _expr: &'a SymbolicExpression, value: &clarity::vm::Value) -> bool {
        match value {
            clarity::vm::Value::Int(i) => {
                self.current_function.func_body().i64_const(*i as i64);
                true
            }
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                false
            }
        }
    }
}
