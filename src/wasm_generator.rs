use std::collections::HashMap;

use clarity::vm::{
    analysis::ContractAnalysis,
    diagnostic::DiagnosableError,
    functions::NativeFunctions,
    types::{FunctionType, TypeSignature},
    SymbolicExpression,
};
use walrus::{FunctionBuilder, FunctionId, LocalId, Module, ValType};

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
        let standard_lib_wasm: &[u8] = include_bytes!("standard/standard.wasm");
        let mut module =
            Module::from_buffer(standard_lib_wasm).expect("failed to load standard library");
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
        // println!("{:?}", expressions);
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
        let mut params_types = Vec::new();
        for param in function_type.args.iter() {
            let param_types = clar2wasm_ty(&param.signature);
            for (n, ty) in param_types.iter().enumerate() {
                let local = self.module.locals.add(*ty);
                self.locals.insert(format!("{}.{}", param.name, n), local);
                param_locals.push(local);
                params_types.push(*ty);
            }
        }

        let mut func_builder = FunctionBuilder::new(
            &mut self.module.types,
            params_types.as_slice(),
            clar2wasm_ty(&function_type.returns).as_slice(),
        );
        func_builder.name(name.as_str().to_string());

        let top_level = std::mem::replace(&mut self.current_function, func_builder);

        self.traverse_expr(body);

        func_builder = std::mem::replace(&mut self.current_function, top_level);

        // Clear the locals hashmap
        self.locals = HashMap::new();

        Some(func_builder.finish(param_locals, &mut self.module.funcs))
    }

    fn add_placeholder_for_type(&mut self, ty: ValType) {
        match ty {
            ValType::I32 => self.current_function.func_body().i32_const(0),
            ValType::I64 => self.current_function.func_body().i64_const(0),
            ValType::F32 => self.current_function.func_body().f32_const(0.0),
            ValType::F64 => self.current_function.func_body().f64_const(0.0),
            ValType::V128 => unimplemented!("V128"),
            ValType::Externref => unimplemented!("Externref"),
            ValType::Funcref => unimplemented!("Funcref"),
        };
    }

    fn get_expr_type(&self, expr: &SymbolicExpression) -> &TypeSignature {
        self.contract_analysis
            .type_map
            .as_ref()
            .expect("type-checker must be called before Wasm generation")
            .get_type(expr)
            .expect("expression must be typed")
    }
}

impl<'a> ASTVisitor<'a> for WasmGenerator {
    fn traverse_arithmetic(
        &mut self,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        operands: &'a [SymbolicExpression],
    ) -> bool {
        let ty = self.get_expr_type(expr);
        let type_suffix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                self.error = Some(GeneratorError::InternalError(
                    "invalid type for arithmetic".to_string(),
                ));
                return false;
            }
        };
        let helper_func = match func {
            NativeFunctions::Add => self
                .module
                .funcs
                .by_name(&format!("add-{type_suffix}"))
                .expect(&format!("function not found: add-{type_suffix}")),
            NativeFunctions::Subtract => self
                .module
                .funcs
                .by_name(&format!("sub-{type_suffix}"))
                .expect(&format!("function not found: sub-{type_suffix}")),
            NativeFunctions::Multiply => self
                .module
                .funcs
                .by_name(&format!("mul-{type_suffix}"))
                .expect(&format!("function not found: mul-{type_suffix}")),
            NativeFunctions::Divide => self
                .module
                .funcs
                .by_name(&format!("div-{type_suffix}"))
                .expect(&format!("function not found: div-{type_suffix}")),
            NativeFunctions::Modulo => self
                .module
                .funcs
                .by_name(&format!("mod-{type_suffix}"))
                .expect(&format!("function not found: mod-{type_suffix}")),
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                return false;
            }
        };

        // Start off with operand 0, then loop over the rest
        if !self.traverse_expr(&operands[0]) {
            return false;
        }
        for operand in operands.iter().skip(1) {
            if !self.traverse_expr(operand) {
                return false;
            }
            self.current_function.func_body().call(helper_func);
        }

        true
    }

    fn visit_literal_value(
        &mut self,
        _expr: &'a SymbolicExpression,
        value: &clarity::vm::Value,
    ) -> bool {
        match value {
            clarity::vm::Value::Int(i) => {
                self.current_function
                    .func_body()
                    .i64_const(((i >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                self.current_function
                    .func_body()
                    .i64_const((i & 0xFFFFFFFFFFFFFFFF) as i64);
                true
            }
            clarity::vm::Value::UInt(u) => {
                self.current_function
                    .func_body()
                    .i64_const(((u >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                self.current_function
                    .func_body()
                    .i64_const((u & 0xFFFFFFFFFFFFFFFF) as i64);
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
        expr: &'a SymbolicExpression,
        atom: &'a clarity::vm::ClarityName,
    ) -> bool {
        // FIXME: This should also handle constants and keywords
        let types = clar2wasm_ty(self.get_expr_type(expr));
        for n in 0..types.len() {
            let local = match self.locals.get(format!("{}.{}", atom.as_str(), n).as_str()) {
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
        }

        true
    }

    fn traverse_define_private(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a clarity::vm::ClarityName,
        _parameters: Option<Vec<crate::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.traverse_define_function(name, body, FunctionKind::Private)
            .is_some()
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

    fn traverse_define_public(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a clarity::vm::ClarityName,
        _parameters: Option<Vec<crate::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        match self.traverse_define_function(name, body, FunctionKind::Public) {
            Some(function_id) => {
                self.module.exports.add(name.as_str(), function_id);
                true
            }
            None => false,
        }
    }

    fn traverse_ok(&mut self, expr: &'a SymbolicExpression, value: &'a SymbolicExpression) -> bool {
        // (ok <val>) is represented by an i32 1, followed by the ok value,
        // followed by a placeholder for the err value
        self.current_function.func_body().i32_const(1);
        if !self.traverse_expr(value) {
            return false;
        }
        let ty = self.get_expr_type(expr);
        if let TypeSignature::ResponseType(inner_types) = ty {
            let err_types = clar2wasm_ty(&inner_types.1);
            for err_type in err_types.iter() {
                self.add_placeholder_for_type(*err_type);
            }
        } else {
            panic!("expected response type");
        }
        true
    }

    fn traverse_err(
        &mut self,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> bool {
        // (err <val>) is represented by an i32 1, followed by a placeholder
        // for the ok value, followed by the err value
        self.current_function.func_body().i32_const(1);
        let ty = self.get_expr_type(expr);
        if let TypeSignature::ResponseType(inner_types) = ty {
            let ok_types = clar2wasm_ty(&inner_types.0);
            for ok_type in ok_types.iter() {
                self.add_placeholder_for_type(*ok_type);
            }
        } else {
            panic!("expected response type");
        }
        self.traverse_expr(value)
    }

    fn visit_call_user_defined(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a clarity::vm::ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> bool {
        self.current_function.func_body().call(
            self.module
                .funcs
                .by_name(name.as_str())
                .expect("function not found"),
        );
        true
    }
}

fn clar2wasm_ty(ty: &TypeSignature) -> Vec<ValType> {
    match ty {
        TypeSignature::NoType => vec![ValType::I32], // TODO: can this just be empty?
        TypeSignature::IntType => vec![ValType::I64, ValType::I64],
        TypeSignature::UIntType => vec![ValType::I64, ValType::I64],
        TypeSignature::ResponseType(inner_types) => {
            let mut types = vec![ValType::I32];
            types.extend(clar2wasm_ty(&inner_types.0));
            types.extend(clar2wasm_ty(&inner_types.1));
            types
        }
        _ => unimplemented!("{:?}", ty),
    }
}
