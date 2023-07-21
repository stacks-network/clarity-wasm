use std::collections::HashMap;

use clarity::vm::{
    analysis::ContractAnalysis,
    diagnostic::DiagnosableError,
    functions::NativeFunctions,
    types::{FunctionType, TypeSignature},
    SymbolicExpression,
};
use walrus::{ir::BinaryOp, FunctionBuilder, FunctionId, LocalId, Module, ModuleConfig, ValType};

use crate::ast_visitor::{traverse, ASTVisitor};

/// WasmGenerator is a Clarity AST visitor that generates a WebAssembly module
/// as it traverses the AST.
pub struct WasmGenerator {
    contract_analysis: ContractAnalysis,
    module: Module,
    error: Option<GeneratorError>,
    /// Current function context we are in.
    current_function: FunctionBuilder,
    /// Locals for the current function.
    locals: HashMap<String, LocalId>,
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

enum FunctionKind {
    Public,
    Private,
    ReadOnly,
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
            locals: HashMap::new(),
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

    fn traverse_define_function(
        &mut self,
        name: &clarity::vm::ClarityName,
        body: &SymbolicExpression,
        kind: FunctionKind,
    ) -> Option<FunctionId> {
        let opt_function_type = match kind {
            FunctionKind::Private => self.contract_analysis.get_private_function(name.as_str()),
            FunctionKind::ReadOnly => self
                .contract_analysis
                .get_read_only_function_type(name.as_str()),
            FunctionKind::Public => self
                .contract_analysis
                .get_public_function_type(name.as_str()),
        };
        let function_type = if let Some(FunctionType::Fixed(fixed)) = opt_function_type {
            fixed
        } else {
            self.error = Some(GeneratorError::InternalError(match opt_function_type {
                Some(_) => "expected fixed function type".to_string(),
                None => format!("unable to find function type for {}", name.as_str()),
            }));
            return None;
        };

        // Ensure the locals hashmap is empty
        assert!(
            self.locals.is_empty(),
            "locals hashmap is not empty at the start of a function"
        );

        // Setup the parameters
        let mut param_locals = Vec::new();
        let mut param_types = Vec::new();
        for param in function_type.args.iter() {
            let local = self.module.locals.add(clar2wasm_ty(&param.signature));
            self.locals.insert(param.name.as_str().to_string(), local);
            param_locals.push(local);
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
        func_builder.name(name.as_str().to_string());

        let top_level = std::mem::replace(&mut self.current_function, func_builder);

        self.traverse_expr(body);

        func_builder = std::mem::replace(&mut self.current_function, top_level);

        // Clear the locals hashmap
        self.locals = HashMap::new();

        Some(func_builder.finish(param_locals, &mut self.module.funcs))
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

    fn visit_atom(
        &mut self,
        _expr: &'a SymbolicExpression,
        atom: &'a clarity::vm::ClarityName,
    ) -> bool {
        // FIXME: This should also handle constants and keywords
        let local = match self.locals.get(atom.as_str()) {
            Some(local) => *local,
            None => {
                self.error = Some(GeneratorError::InternalError(format!(
                    "unable to find local for {}",
                    atom.as_str()
                )));
                return false;
            }
        };
        self.current_function.func_body().local_get(local);
        true
    }

    fn traverse_define_private(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a clarity::vm::ClarityName,
        _parameters: Option<Vec<crate::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        match self.traverse_define_function(name, body, FunctionKind::Private) {
            Some(_) => true,
            None => false,
        }
    }

    fn traverse_define_read_only(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a clarity::vm::ClarityName,
        _parameters: Option<Vec<crate::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        match self.traverse_define_function(name, body, FunctionKind::ReadOnly) {
            Some(function_id) => {
                self.module.exports.add(name.as_str(), function_id);
                true
            }
            None => false,
        }
    }
}

fn clar2wasm_ty(ty: &TypeSignature) -> ValType {
    match ty {
        TypeSignature::IntType => ValType::I64,
        _ => unimplemented!(),
    }
}
