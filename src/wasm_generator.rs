use clarity::vm::{
    analysis::ContractAnalysis,
    diagnostic::DiagnosableError,
    functions::NativeFunctions,
    types::{FunctionType, TypeSignature},
    SymbolicExpression,
};
use walrus::{ir::BinaryOp, FunctionBuilder, Module, ModuleConfig, ValType};

use crate::ast_visitor::{traverse, ASTVisitor};

pub struct WasmGenerator {
    contract_analysis: ContractAnalysis,
    module: Module,
    error: Option<GeneratorError>,
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
    pub fn new(contract_analysis: ContractAnalysis) -> WasmGenerator {
        let mut module = Module::with_config(ModuleConfig::default());
        let top_level = FunctionBuilder::new(&mut module.types, &[], &[]);

        WasmGenerator {
            contract_analysis,
            module,
            error: None,
            current_function: top_level,
        }
    }

    pub fn generate(mut self) -> Result<Module, GeneratorError> {
        let expressions = std::mem::replace(&mut self.contract_analysis.expressions, vec![]);
        traverse(&mut self, &expressions);
        println!("{:?}", expressions);
        self.contract_analysis.expressions = expressions;

        if let Some(err) = self.error {
            return Err(err);
        }

        // Insert a return instruction at the end of the top-level function so
        // that the top level always has no return value.
        self.current_function.func_body().return_();

        self.module.exports.add(
            ".top-level",
            self.current_function.finish(vec![], &mut self.module.funcs),
        );

        Ok(self.module)
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

    fn visit_literal_value(
        &mut self,
        _expr: &'a SymbolicExpression,
        value: &clarity::vm::Value,
    ) -> bool {
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

    // fn visit_atom(&mut self, _expr: &'a SymbolicExpression, _atom: &'a clarity::vm::ClarityName) -> bool {
    //     self.current_function.func_body().local_get(local)
    // }

    fn traverse_define_private(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a clarity::vm::ClarityName,
        _parameters: Option<Vec<crate::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        let function_type = match self.contract_analysis.get_private_function(name.as_str()) {
            Some(function_type) => match function_type {
                FunctionType::Fixed(fixed) => fixed,
                _ => {
                    self.error = Some(GeneratorError::NotImplemented);
                    return false;
                }
            },
            None => {
                self.error = Some(GeneratorError::InternalError(format!(
                    "unable to find function type for {}",
                    name.as_str()
                )));
                return false;
            }
        };

        // TODO: Create locals for the parameters
        let mut param_locals = Vec::new();
        let mut param_types = Vec::new();
        for param in function_type.args.iter() {
            param_locals.push(self.module.locals.add(clar2wasm_ty(&param.signature)));
            param_types.push(clar2wasm_ty(&param.signature));
        }

        let mut func_builder = FunctionBuilder::new(
            &mut self.module.types,
            function_type
                .args
                .iter()
                .map(|arg| clar2wasm_ty(&arg.signature))
                .collect::<Vec<_>>()
                .as_slice(),
            &[clar2wasm_ty(&function_type.returns)],
        );

        let top_level = std::mem::replace(&mut self.current_function, func_builder);

        self.traverse_expr(body);

        func_builder = std::mem::replace(&mut self.current_function, top_level);

        func_builder.finish(param_locals, &mut self.module.funcs);

        true
    }
}

fn clar2wasm_ty(ty: &TypeSignature) -> ValType {
    match ty {
        TypeSignature::IntType => ValType::I64,
        _ => unimplemented!(),
    }
}
