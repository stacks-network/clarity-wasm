use clarity::vm::functions::define::DefineFunctions;
use clarity::vm::types::{QualifiedContractIdentifier, TraitIdentifier};
use clarity::vm::ClarityVersion;
use clarity::vm::{
    analysis::ContractAnalysis,
    diagnostic::DiagnosableError,
    functions::NativeFunctions,
    representations::{Span, TraitDefinition},
    types::{
        CharType, FunctionType, PrincipalData, SequenceData, SequenceSubtype, StringSubtype,
        TypeSignature,
    },
    variables::NativeVariables,
    ClarityName, SymbolicExpression, SymbolicExpressionType, Value,
};
use lazy_static::lazy_static;
use std::{borrow::BorrowMut, collections::HashMap};
use walrus::{
    ir::{BinaryOp, Block, InstrSeqType, LoadKind, MemArg, StoreKind, UnaryOp},
    ActiveData, DataKind, FunctionBuilder, FunctionId, GlobalId, InstrSeqBuilder, LocalId, Module,
    ValType,
};

pub type CResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct GenericError;

impl std::fmt::Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error")
    }
}

impl std::error::Error for GenericError {}

lazy_static! {
    // Since the AST Visitor may be used before other checks have been performed,
    // we may need a default value for some expressions. This can be used for a
    // missing `ClarityName`.
    static ref DEFAULT_NAME: ClarityName = ClarityName::from("placeholder__");
    static ref DEFAULT_EXPR: SymbolicExpression = SymbolicExpression::atom(DEFAULT_NAME.clone());
}

/// `Trap` should match the values used in the standard library and is used to
/// indicate the reason for a runtime error from the Clarity code.
#[allow(dead_code)]
#[repr(i32)]
enum Trap {
    Overflow = 0,
    Underflow = 1,
    DivideByZero = 2,
    LogOfNumberLessThanOrEqualToZero = 3,
    ExpectedANonNegativeNumber = 4,
    Panic = 5,
}

#[derive(Clone)]
pub struct TypedVar<'a> {
    pub name: &'a ClarityName,
    pub type_expr: &'a SymbolicExpression,
    pub decl_span: Span,
}

/// WasmGenerator is a Clarity AST visitor that generates a WebAssembly module
/// as it traverses the AST.
pub struct WasmGenerator {
    /// The contract analysis, which contains the expressions and type
    /// information for the contract.
    contract_analysis: ContractAnalysis,
    /// The WebAssembly module that is being generated.
    module: Module,
    /// The error that occurred during generation, if any.
    error: Option<GeneratorError>,
    /// Offset of the end of the literal memory.
    literal_memory_end: u32,
    /// Global ID of the stack pointer.
    stack_pointer: GlobalId,
    /// Map strings saved in the literal memory to their offset.
    literal_memory_offet: HashMap<String, u32>,
    /// Map constants to an offset in the literal memory.
    constants: HashMap<String, u32>,

    /// The locals for the current function.
    locals: HashMap<String, LocalId>,
    /// Size of the current function's stack frame.
    frame_size: i32,
}

#[derive(Debug)]
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

/// Return the number of bytes required to store a value of the type `ty`.
fn get_type_size(ty: &TypeSignature) -> u32 {
    match ty {
        TypeSignature::IntType | TypeSignature::UIntType => 16,
        TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(length))) => {
            u32::from(length)
        }
        TypeSignature::PrincipalType => {
            // Standard principal is a 1 byte version and a 20 byte Hash160.
            // Then there is an int32 for the contract name length, followed by
            // the contract name, which has a max length of 128.
            1 + 20 + 1 + 128
        }
        TypeSignature::OptionalType(inner) => 1 + get_type_size(inner),
        TypeSignature::SequenceType(SequenceSubtype::ListType(list_data)) => {
            list_data.get_max_len() * get_type_size(list_data.get_list_item_type())
        }
        TypeSignature::SequenceType(SequenceSubtype::BufferType(length)) => u32::from(length),
        _ => unimplemented!("Unsupported type: {}", ty),
    }
}

/// Return true if the value of the given type stays in memory, and false if
/// it is stored on the data stack.
fn is_in_memory_type(ty: &TypeSignature) -> bool {
    match ty {
        TypeSignature::PrincipalType | TypeSignature::SequenceType(_) => true,
        TypeSignature::IntType
        | TypeSignature::UIntType
        | TypeSignature::NoType
        | TypeSignature::BoolType
        | TypeSignature::TupleType(_)
        | TypeSignature::OptionalType(_)
        | TypeSignature::ResponseType(_) => false,
        _ => todo!("Unsupported type: {}", ty),
    }
}

/// Push a placeholder value for Wasm type `ty` onto the data stack.
fn add_placeholder_for_type(builder: &mut InstrSeqBuilder, ty: ValType) {
    match ty {
        ValType::I32 => builder.i32_const(0),
        ValType::I64 => builder.i64_const(0),
        ValType::F32 => builder.f32_const(0.0),
        ValType::F64 => builder.f64_const(0.0),
        ValType::V128 => unimplemented!("V128"),
        ValType::Externref => unimplemented!("Externref"),
        ValType::Funcref => unimplemented!("Funcref"),
    };
}

/// Push a placeholder value for Clarity type `ty` onto the data stack.
fn add_placeholder_for_clarity_type(builder: &mut InstrSeqBuilder, ty: &TypeSignature) {
    let wasm_types = clar2wasm_ty(ty);
    for wasm_type in wasm_types.iter() {
        add_placeholder_for_type(builder, *wasm_type);
    }
}

pub fn traverse<'b>(
    visitor: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder<'b>,
    exprs: &[SymbolicExpression],
) -> CResult<()> {
    for expr in exprs {
        visitor.traverse_expr(builder, expr)?;
    }
    Ok(())
}

impl WasmGenerator {
    pub fn new(contract_analysis: ContractAnalysis) -> WasmGenerator {
        let standard_lib_wasm: &[u8] = include_bytes!("standard/standard.wasm");
        let module =
            Module::from_buffer(standard_lib_wasm).expect("failed to load standard library");

        // Get the stack-pointer global ID
        let stack_pointer_name = "stack-pointer";
        let global_id = module
            .globals
            .iter()
            .find(|global| {
                global
                    .name
                    .as_ref()
                    .map_or(false, |name| name == stack_pointer_name)
            })
            .map(|global| global.id())
            .expect("Expected to find a global named $stack-pointer");

        WasmGenerator {
            contract_analysis,
            module,
            error: None,
            literal_memory_end: 0,
            stack_pointer: global_id,
            literal_memory_offet: HashMap::new(),
            constants: HashMap::new(),
            locals: HashMap::new(),
            frame_size: 0,
        }
    }

    pub fn generate(mut self) -> Result<Module, GeneratorError> {
        let expressions = std::mem::take(&mut self.contract_analysis.expressions);
        // println!("{:?}", expressions);

        let mut current_function = FunctionBuilder::new(&mut self.module.types, &[], &[]);

        if traverse(&mut self, &mut current_function.func_body(), &expressions).is_err() {
            return Err(GeneratorError::InternalError(
                "ast traversal failed".to_string(),
            ));
        }

        self.contract_analysis.expressions = expressions;

        if let Some(err) = self.error {
            return Err(err);
        }

        // Insert a return instruction at the end of the top-level function so
        // that the top level always has no return value.
        current_function.func_body().return_();
        let top_level = current_function.finish(vec![], &mut self.module.funcs);
        self.module.exports.add(".top-level", top_level);

        // Update the initial value of the stack-pointer to point beyond the
        // literal memory.
        self.module.globals.get_mut(self.stack_pointer).kind = walrus::GlobalKind::Local(
            walrus::InitExpr::Value(walrus::ir::Value::I32(self.literal_memory_end as i32)),
        );

        Ok(self.module)
    }

    fn traverse_expr<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
    ) -> CResult<()> {
        match &expr.expr {
            SymbolicExpressionType::AtomValue(value) => self.visit_atom_value(builder, expr, value),
            SymbolicExpressionType::Atom(name) => self.visit_atom(builder, expr, name),
            SymbolicExpressionType::List(exprs) => self.traverse_list(builder, expr, exprs),
            SymbolicExpressionType::LiteralValue(value) => {
                self.visit_literal_value(builder, expr, value)
            }
            SymbolicExpressionType::Field(field) => self.visit_field(builder, expr, field),
            SymbolicExpressionType::TraitReference(name, trait_def) => {
                self.visit_trait_reference(builder, expr, name, trait_def)
            }
        }
    }

    fn traverse_list<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        list: &[SymbolicExpression],
    ) -> CResult<()> {
        if let Some((function_name, args)) = list.split_first() {
            if let Some(function_name) = function_name.match_atom() {
                if let Some(define_function) = DefineFunctions::lookup_by_name(function_name) {
                    match define_function {
                        DefineFunctions::Constant => self.traverse_define_constant(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        DefineFunctions::PrivateFunction
                        | DefineFunctions::ReadOnlyFunction
                        | DefineFunctions::PublicFunction => {
                            match args.get(0).unwrap_or(&DEFAULT_EXPR).match_list() {
                                Some(signature) => {
                                    let name = signature
                                        .get(0)
                                        .and_then(|n| n.match_atom())
                                        .unwrap_or(&DEFAULT_NAME);
                                    let params = match signature.len() {
                                        0 | 1 => None,
                                        _ => match_pairs_list(&signature[1..]),
                                    };
                                    let body = args.get(1).unwrap_or(&DEFAULT_EXPR);

                                    match define_function {
                                        DefineFunctions::PrivateFunction => self
                                            .traverse_define_private(
                                                builder, expr, name, params, body,
                                            ),
                                        DefineFunctions::ReadOnlyFunction => self
                                            .traverse_define_read_only(
                                                builder, expr, name, params, body,
                                            ),
                                        DefineFunctions::PublicFunction => self
                                            .traverse_define_public(
                                                builder, expr, name, params, body,
                                            ),
                                        _ => unreachable!(),
                                    }
                                }
                                _ => Err(Box::new(GenericError) as Box<dyn std::error::Error>),
                            }
                        }
                        DefineFunctions::NonFungibleToken => self.traverse_define_nft(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        DefineFunctions::FungibleToken => self.traverse_define_ft(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1),
                        ),
                        DefineFunctions::Map => self.traverse_define_map(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        DefineFunctions::PersistedVariable => self.traverse_define_data_var(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        DefineFunctions::Trait => {
                            let params = if !args.is_empty() { &args[1..] } else { &[] };
                            self.traverse_define_trait(
                                builder,
                                expr,
                                args.get(0)
                                    .unwrap_or(&DEFAULT_EXPR)
                                    .match_atom()
                                    .unwrap_or(&DEFAULT_NAME),
                                params,
                            )
                        }
                        DefineFunctions::UseTrait => self.traverse_use_trait(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_field()
                                .unwrap_or(&TraitIdentifier {
                                    contract_identifier: QualifiedContractIdentifier::transient(),
                                    name: DEFAULT_NAME.clone(),
                                }),
                        ),
                        DefineFunctions::ImplTrait => self.traverse_impl_trait(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_field()
                                .unwrap_or(&TraitIdentifier {
                                    contract_identifier: QualifiedContractIdentifier::transient(),
                                    name: DEFAULT_NAME.clone(),
                                }),
                        ),
                    }?;
                } else if let Some(native_function) = NativeFunctions::lookup_by_name_at_version(
                    function_name,
                    &ClarityVersion::latest(), // FIXME(brice): this should probably be passed in
                ) {
                    use clarity::vm::functions::NativeFunctions::*;
                    match native_function {
                        Add | Subtract | Multiply | Divide | Modulo | Power | Sqrti | Log2 => {
                            self.traverse_arithmetic(builder, expr, native_function, args)
                        }
                        BitwiseXor => self.traverse_binary_bitwise(
                            builder,
                            expr,
                            native_function,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        CmpLess | CmpLeq | CmpGreater | CmpGeq | Equals => {
                            self.traverse_comparison(builder, expr, native_function, args)
                        }
                        And | Or => {
                            self.traverse_lazy_logical(builder, expr, native_function, args)
                        }
                        Not => self.traverse_logical(builder, expr, native_function, args),
                        ToInt | ToUInt => self.traverse_int_cast(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        If => self.traverse_if(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Let => {
                            let bindings = match_pairs(args.get(0).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_default();
                            let params = if !args.is_empty() { &args[1..] } else { &[] };
                            self.traverse_let(builder, expr, &bindings, params)
                        }
                        ElementAt | ElementAtAlias => self.traverse_element_at(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        IndexOf | IndexOfAlias => self.traverse_index_of(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Map => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            let params = if !args.is_empty() { &args[1..] } else { &[] };
                            self.traverse_map(builder, expr, name, params)
                        }
                        Fold => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            self.traverse_fold(
                                builder,
                                expr,
                                name,
                                args.get(1).unwrap_or(&DEFAULT_EXPR),
                                args.get(2).unwrap_or(&DEFAULT_EXPR),
                            )
                        }
                        Append => self.traverse_append(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Concat => self.traverse_concat(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        AsMaxLen => {
                            match args.get(1).unwrap_or(&DEFAULT_EXPR).match_literal_value() {
                                Some(Value::UInt(length)) => self.traverse_as_max_len(
                                    builder,
                                    expr,
                                    args.get(0).unwrap_or(&DEFAULT_EXPR),
                                    *length,
                                ),
                                _ => Err(Box::new(GenericError) as Box<dyn std::error::Error>),
                            }
                        }
                        Len => {
                            self.traverse_len(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        ListCons => self.traverse_list_cons(builder, expr, args),
                        FetchVar => self.traverse_var_get(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                        ),
                        SetVar => self.traverse_var_set(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        FetchEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            self.traverse_map_get(
                                builder,
                                expr,
                                name,
                                args.get(1).unwrap_or(&DEFAULT_EXPR),
                            )
                        }
                        SetEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            self.traverse_map_set(
                                builder,
                                expr,
                                name,
                                args.get(1).unwrap_or(&DEFAULT_EXPR),
                                args.get(2).unwrap_or(&DEFAULT_EXPR),
                            )
                        }
                        InsertEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            self.traverse_map_insert(
                                builder,
                                expr,
                                name,
                                args.get(1).unwrap_or(&DEFAULT_EXPR),
                                args.get(2).unwrap_or(&DEFAULT_EXPR),
                            )
                        }
                        DeleteEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            self.traverse_map_delete(
                                builder,
                                expr,
                                name,
                                args.get(1).unwrap_or(&DEFAULT_EXPR),
                            )
                        }
                        TupleCons => self.traverse_tuple(
                            builder,
                            expr,
                            &match_tuple(expr).unwrap_or_default(),
                        ),
                        TupleGet => self.traverse_get(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        TupleMerge => self.traverse_merge(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Begin => self.traverse_begin(builder, expr, args),
                        Hash160 | Sha256 | Sha512 | Sha512Trunc256 | Keccak256 => self
                            .traverse_hash(
                                builder,
                                expr,
                                native_function,
                                args.get(0).unwrap_or(&DEFAULT_EXPR),
                            ),
                        Secp256k1Recover => self.traverse_secp256k1_recover(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Secp256k1Verify => self.traverse_secp256k1_verify(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Print => {
                            self.traverse_print(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        ContractCall => {
                            let function_name = args
                                .get(1)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            let params = if args.len() >= 2 { &args[2..] } else { &[] };
                            if let SymbolicExpressionType::LiteralValue(Value::Principal(
                                PrincipalData::Contract(ref contract_identifier),
                            )) = args.get(0).unwrap_or(&DEFAULT_EXPR).expr
                            {
                                self.traverse_static_contract_call(
                                    builder,
                                    expr,
                                    contract_identifier,
                                    function_name,
                                    params,
                                )
                            } else {
                                self.traverse_dynamic_contract_call(
                                    builder,
                                    expr,
                                    args.get(0).unwrap_or(&DEFAULT_EXPR),
                                    function_name,
                                    params,
                                )
                            }
                        }
                        AsContract => self.traverse_as_contract(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        ContractOf => self.traverse_contract_of(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        PrincipalOf => self.traverse_principal_of(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        AtBlock => self.traverse_at_block(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        GetBlockInfo => self.traverse_get_block_info(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        ConsError => {
                            self.traverse_err(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        ConsOkay => {
                            self.traverse_ok(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        ConsSome => {
                            self.traverse_some(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        DefaultTo => self.traverse_default_to(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Asserts => self.traverse_asserts(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        UnwrapRet => self.traverse_unwrap(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Unwrap => self.traverse_unwrap_panic(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        IsOkay => {
                            self.traverse_is_ok(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        IsNone => self.traverse_is_none(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        IsErr => self.traverse_is_err(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        IsSome => self.traverse_is_some(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Filter => self.traverse_filter(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        UnwrapErrRet => self.traverse_unwrap_err(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        UnwrapErr => self.traverse_unwrap_err(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Match => {
                            if args.len() == 4 {
                                self.traverse_match_option(
                                    builder,
                                    expr,
                                    args.get(0).unwrap_or(&DEFAULT_EXPR),
                                    args.get(1)
                                        .unwrap_or(&DEFAULT_EXPR)
                                        .match_atom()
                                        .unwrap_or(&DEFAULT_NAME),
                                    args.get(2).unwrap_or(&DEFAULT_EXPR),
                                    args.get(3).unwrap_or(&DEFAULT_EXPR),
                                )
                            } else {
                                self.traverse_match_response(
                                    builder,
                                    expr,
                                    args.get(0).unwrap_or(&DEFAULT_EXPR),
                                    args.get(1)
                                        .unwrap_or(&DEFAULT_EXPR)
                                        .match_atom()
                                        .unwrap_or(&DEFAULT_NAME),
                                    args.get(2).unwrap_or(&DEFAULT_EXPR),
                                    args.get(3)
                                        .unwrap_or(&DEFAULT_EXPR)
                                        .match_atom()
                                        .unwrap_or(&DEFAULT_NAME),
                                    args.get(4).unwrap_or(&DEFAULT_EXPR),
                                )
                            }
                        }
                        TryRet => {
                            self.traverse_try(builder, expr, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        StxBurn => self.traverse_stx_burn(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        StxTransfer | StxTransferMemo => self.traverse_stx_transfer(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                            args.get(3),
                        ),
                        GetStxBalance => self.traverse_stx_get_balance(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        BurnToken => self.traverse_ft_burn(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        TransferToken => self.traverse_ft_transfer(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                            args.get(3).unwrap_or(&DEFAULT_EXPR),
                        ),
                        GetTokenBalance => self.traverse_ft_get_balance(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        GetTokenSupply => self.traverse_ft_get_supply(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                        ),
                        MintToken => self.traverse_ft_mint(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        BurnAsset => self.traverse_nft_burn(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        TransferAsset => self.traverse_nft_transfer(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                            args.get(3).unwrap_or(&DEFAULT_EXPR),
                        ),
                        MintAsset => self.traverse_nft_mint(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        GetAssetOwner => self.traverse_nft_get_owner(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        BuffToIntLe | BuffToUIntLe | BuffToIntBe | BuffToUIntBe => self
                            .traverse_buff_cast(
                                builder,
                                expr,
                                args.get(0).unwrap_or(&DEFAULT_EXPR),
                            ),
                        IsStandard => self.traverse_is_standard(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        PrincipalDestruct => self.traverse_principal_destruct(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        PrincipalConstruct => self.traverse_principal_construct(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2),
                        ),
                        StringToInt | StringToUInt => self.traverse_string_to_int(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        IntToAscii | IntToUtf8 => self.traverse_int_to_string(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        GetBurnBlockInfo => self.traverse_get_burn_block_info(
                            builder,
                            expr,
                            args.get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        StxGetAccount => self.traverse_stx_get_account(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        Slice => self.traverse_slice(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        ToConsensusBuff => self.traverse_to_consensus_buff(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                        ),
                        FromConsensusBuff => self.traverse_from_consensus_buff(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                        ReplaceAt => self.traverse_replace_at(
                            builder,
                            expr,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                            args.get(2).unwrap_or(&DEFAULT_EXPR),
                        ),
                        BitwiseAnd | BitwiseOr | BitwiseXor2 => {
                            self.traverse_bitwise(builder, expr, native_function, args)
                        }
                        BitwiseNot => {
                            self.traverse_bitwise_not(builder, args.get(0).unwrap_or(&DEFAULT_EXPR))
                        }
                        BitwiseLShift | BitwiseRShift => self.traverse_bit_shift(
                            builder,
                            expr,
                            native_function,
                            args.get(0).unwrap_or(&DEFAULT_EXPR),
                            args.get(1).unwrap_or(&DEFAULT_EXPR),
                        ),
                    }?;
                } else {
                    self.traverse_call_user_defined(builder, expr, function_name, args)?;
                }
            }
        }
        self.visit_list(builder, expr, list)
    }

    fn visit_list<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _list: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_define_function(
        &mut self,
        builder: &mut InstrSeqBuilder,
        name: &ClarityName,
        body: &SymbolicExpression,
        kind: FunctionKind,
    ) -> Option<FunctionId> {
        let opt_function_type = match kind {
            FunctionKind::ReadOnly => {
                builder.i32_const(0);
                self.contract_analysis
                    .get_read_only_function_type(name.as_str())
            }
            FunctionKind::Public => {
                builder.i32_const(1);
                self.contract_analysis
                    .get_public_function_type(name.as_str())
            }
            FunctionKind::Private => {
                builder.i32_const(2);
                self.contract_analysis.get_private_function(name.as_str())
            }
        };
        let function_type = if let Some(FunctionType::Fixed(fixed)) = opt_function_type {
            fixed.clone()
        } else {
            self.error = Some(GeneratorError::InternalError(match opt_function_type {
                Some(_) => "expected fixed function type".to_string(),
                None => format!("unable to find function type for {}", name.as_str()),
            }));
            return None;
        };

        // Call the host interface to save this function
        // Arguments are kind (already pushed) and name (offset, length)
        let (id_offset, id_length) = self.add_identifier_string_literal(name);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Call the host interface function, `define_function`
        builder.call(
            self.module
                .funcs
                .by_name("define_function")
                .expect("define_function not found"),
        );

        let mut locals = HashMap::new();

        // Setup the parameters
        let mut param_locals = Vec::new();
        let mut params_types = Vec::new();
        for param in function_type.args.iter() {
            let param_types = clar2wasm_ty(&param.signature);
            for (n, ty) in param_types.iter().enumerate() {
                let local = self.module.locals.add(*ty);
                locals.insert(format!("{}.{}", param.name, n), local);
                param_locals.push(local);
                params_types.push(*ty);
            }
        }

        let results_types = clar2wasm_ty(&function_type.returns);
        let mut func_builder = FunctionBuilder::new(
            &mut self.module.types,
            params_types.as_slice(),
            results_types.as_slice(),
        );
        func_builder.name(name.as_str().to_string());
        let mut func_body = func_builder.func_body();

        // Function prelude
        // Save the frame pointer in a local variable.
        let frame_pointer = self.module.locals.add(ValType::I32);
        func_body
            .global_get(self.stack_pointer)
            .local_set(frame_pointer);

        // Setup the locals map for this function, saving the top-level map to
        // restore after.
        let top_level_locals = std::mem::replace(&mut self.locals, locals);

        let mut block = func_body.dangling_instr_seq(InstrSeqType::new(
            &mut self.module.types,
            &[],
            results_types.as_slice(),
        ));
        let block_id = block.id();

        // Traverse the body of the function
        if self.traverse_expr(&mut block, body).is_err() {
            return None;
        }

        // TODO: We need to ensure that all exits from the function go through
        // the postlude. Maybe put the body in a block, and then have any exits
        // from the block go to the postlude with a `br` instruction?

        // Insert the function body block into the function
        func_body.instr(Block { seq: block_id });

        // Function postlude
        // Restore the initial stack pointer.
        func_body
            .local_get(frame_pointer)
            .global_set(self.stack_pointer);

        // Restore the top-level locals map.
        self.locals = top_level_locals;

        Some(func_builder.finish(param_locals, &mut self.module.funcs))
    }

    /// Gets the result type of the given `SymbolicExpression`.
    fn get_expr_type(&self, expr: &SymbolicExpression) -> Option<&TypeSignature> {
        self.contract_analysis
            .type_map
            .as_ref()
            .expect("type-checker must be called before Wasm generation")
            .get_type(expr)
    }

    fn visit_atom_value<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &Value,
    ) -> CResult<()> {
        Ok(())
    }

    /// Adds a new string literal into the memory, and returns the offset and length.
    fn add_string_literal(&mut self, s: &CharType) -> (u32, u32) {
        // If this string has already been saved in the literal memory,
        // just return the offset and length.
        if let Some(offset) = self.literal_memory_offet.get(s.to_string().as_str()) {
            return (*offset, s.to_string().len() as u32);
        }

        let data = match s {
            CharType::ASCII(s) => s.data.clone(),
            CharType::UTF8(u) => u.data.clone().into_iter().flatten().collect(),
        };
        let memory = self.module.memories.iter().next().expect("no memory found");
        let offset = self.literal_memory_end;
        let len = data.len() as u32;
        self.module.data.add(
            DataKind::Active(ActiveData {
                memory: memory.id(),
                location: walrus::ActiveDataLocation::Absolute(offset),
            }),
            data.clone(),
        );
        self.literal_memory_end += data.len() as u32;

        // Save the offset in the literal memory for this string
        self.literal_memory_offet.insert(s.to_string(), offset);

        (offset, len)
    }

    /// Adds a new string literal into the memory for an identifier
    fn add_identifier_string_literal(&mut self, name: &clarity::vm::ClarityName) -> (u32, u32) {
        // If this identifier has already been saved in the literal memory,
        // just return the offset and length.
        if let Some(offset) = self.literal_memory_offet.get(name.as_str()) {
            return (*offset, name.len() as u32);
        }

        let memory = self.module.memories.iter().next().expect("no memory found");
        let offset = self.literal_memory_end;
        let len = name.len() as u32;
        self.module.data.add(
            DataKind::Active(ActiveData {
                memory: memory.id(),
                location: walrus::ActiveDataLocation::Absolute(offset),
            }),
            name.as_bytes().to_vec(),
        );
        self.literal_memory_end += name.len() as u32;

        // Save the offset in the literal memory for this identifier
        self.literal_memory_offet.insert(name.to_string(), offset);

        (offset, len)
    }

    /// Adds a new literal into the memory, and returns the offset and length.
    fn add_literal(&mut self, value: &clarity::vm::Value) -> (u32, u32) {
        let data = match value {
            clarity::vm::Value::Int(i) => {
                let mut data = (((*i as u128) & 0xFFFFFFFFFFFFFFFF) as i64)
                    .to_le_bytes()
                    .to_vec();
                data.extend_from_slice(&(((*i as u128) >> 64) as i64).to_le_bytes());
                data
            }
            clarity::vm::Value::UInt(u) => {
                let mut data = ((*u & 0xFFFFFFFFFFFFFFFF) as i64).to_le_bytes().to_vec();
                data.extend_from_slice(&((*u >> 64) as i64).to_le_bytes());
                data
            }
            clarity::vm::Value::Principal(p) => match p {
                PrincipalData::Standard(standard) => {
                    let mut data = vec![standard.0];
                    data.extend_from_slice(&standard.1);
                    let contract_length = 0i32.to_le_bytes();
                    data.extend_from_slice(&contract_length);
                    data
                }
                PrincipalData::Contract(contract) => {
                    let mut data = vec![contract.issuer.0];
                    data.extend_from_slice(&contract.issuer.1);
                    let contract_length = (contract.name.len() as i32).to_le_bytes();
                    data.extend_from_slice(&contract_length);
                    data.extend_from_slice(contract.name.as_bytes());
                    data
                }
            },
            clarity::vm::Value::Sequence(SequenceData::Buffer(buff_data)) => buff_data.data.clone(),
            clarity::vm::Value::Sequence(SequenceData::String(string_data)) => {
                return self.add_string_literal(string_data);
            }
            _ => unimplemented!("Unsupported literal: {}", value),
        };
        let memory = self.module.memories.iter().next().expect("no memory found");
        let offset = self.literal_memory_end;
        let len = data.len() as u32;
        self.module.data.add(
            DataKind::Active(ActiveData {
                memory: memory.id(),
                location: walrus::ActiveDataLocation::Absolute(offset),
            }),
            data.clone(),
        );
        self.literal_memory_end += data.len() as u32;

        (offset, len)
    }

    /// Push a new local onto the call stack, adjusting the stack pointer and
    /// tracking the current function's frame size accordingly.
    fn create_call_stack_local<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        stack_pointer: GlobalId,
        ty: &TypeSignature,
    ) -> (LocalId, i32) {
        let size = get_type_size(ty) as i32;

        // Save the offset (current stack pointer) into a local
        let offset = self.module.locals.add(ValType::I32);
        builder.global_get(stack_pointer).local_tee(offset);

        // TODO: The frame stack size can be computed at compile time, so we
        //       should be able to increment the stack pointer once in the function
        //       prelude with a constant instead of incrementing it for each local.
        // (global.set $stack-pointer (i32.add (global.get $stack-pointer) (i32.const <size>))
        builder
            .i32_const(size)
            .binop(BinaryOp::I32Add)
            .global_set(stack_pointer);
        self.frame_size += size;

        (offset, size)
    }

    /// Write the value that is on the top of the data stack, which has type
    /// `ty`, to the memory, at offset stored in local variable,
    /// `offset_local`, plus constant offset `offset`.
    fn write_to_memory(
        &mut self,
        builder: &mut InstrSeqBuilder,
        offset_local: LocalId,
        offset: u32,
        ty: &TypeSignature,
    ) -> i32 {
        let memory = self.module.memories.iter().next().expect("no memory found");
        let size = match ty {
            TypeSignature::IntType | TypeSignature::UIntType => {
                // Data stack: TOP | Low | High | ...
                // Save the high/low to locals.
                let high = self.module.locals.add(ValType::I64);
                let low = self.module.locals.add(ValType::I64);
                builder.local_set(low).local_set(high);

                // Store the high/low to memory.
                builder.local_get(offset_local).local_get(high).store(
                    memory.id(),
                    StoreKind::I64 { atomic: false },
                    MemArg { align: 8, offset },
                );
                builder.local_get(offset_local).local_get(low).store(
                    memory.id(),
                    StoreKind::I64 { atomic: false },
                    MemArg {
                        align: 8,
                        offset: offset + 8,
                    },
                );
                16
            }
            _ => unimplemented!("Type not yet supported for writing to memory: {ty}"),
        };
        size
    }

    /// Read a value from memory at offset stored in local variable `offset`,
    /// with type `ty`, and push it onto the top of the data stack.
    fn read_from_memory(
        &mut self,
        builder: &mut InstrSeqBuilder,
        offset: LocalId,
        literal_offset: u32,
        ty: &TypeSignature,
    ) -> i32 {
        let memory = self.module.memories.iter().next().expect("no memory found");
        let size = match ty {
            TypeSignature::IntType | TypeSignature::UIntType => {
                // Memory: Offset -> | Low | High |
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I64 { atomic: false },
                    MemArg {
                        align: 8,
                        offset: literal_offset,
                    },
                );
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I64 { atomic: false },
                    MemArg {
                        align: 8,
                        offset: literal_offset + 8,
                    },
                );
                16
            }
            TypeSignature::OptionalType(inner) => {
                // Memory: Offset -> | Indicator | Value |
                builder.local_get(offset).load(
                    memory.id(),
                    LoadKind::I32 { atomic: false },
                    MemArg {
                        align: 4,
                        offset: literal_offset,
                    },
                );
                4 + self.read_from_memory(builder, offset, literal_offset + 4, inner)
            }
            // For types that are represented in-memory, just return the offset
            // and length.
            TypeSignature::PrincipalType | TypeSignature::SequenceType(_) => {
                if literal_offset > 0 {
                    builder.i32_const(literal_offset as i32);
                    builder.local_get(offset);
                    builder.binop(BinaryOp::I32Add);
                } else {
                    builder.local_get(offset);
                }
                let len = get_type_size(ty) as i32;
                builder.i32_const(len);
                len
            }
            _ => unimplemented!("Type not yet supported for reading from memory: {ty}"),
        };
        size
    }

    fn traverse_statement_list<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        statements: &[SymbolicExpression],
    ) -> CResult<()> {
        assert!(
            statements.len() > 1,
            "statement list must have at least one statement"
        );
        // Traverse all but the last statement and drop any unused values.
        for stmt in &statements[..statements.len() - 1] {
            self.traverse_expr(builder, stmt)?;
            // If stmt has a type, and is not the last statement, its value
            // needs to be discarded.
            if let Some(ty) = self.get_expr_type(stmt) {
                drop_value(builder.borrow_mut(), ty);
            }
        }

        // Traverse the last statement in the block, whose result is the result
        // of the `begin` expression.
        self.traverse_expr(builder, statements.last().unwrap())
    }

    /// If `name` is a reserved variable, push its value onto the data stack.
    pub fn lookup_reserved_variable<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        name: &str,
        ty: &TypeSignature,
    ) -> bool {
        if let Some(variable) = NativeVariables::lookup_by_name_at_version(
            name,
            &self.contract_analysis.clarity_version,
        ) {
            match variable {
                NativeVariables::TxSender => {
                    // Create a new local to hold the result on the call stack
                    let (offset, size);
                    (offset, size) = self.create_call_stack_local(
                        builder,
                        self.stack_pointer,
                        &TypeSignature::PrincipalType,
                    );

                    // Push the offset and size to the data stack
                    builder.local_get(offset).i32_const(size);

                    // Call the host interface function, `tx_sender`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("tx_sender")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::ContractCaller => {
                    // Create a new local to hold the result on the call stack
                    let (offset, size);
                    (offset, size) = self.create_call_stack_local(
                        builder,
                        self.stack_pointer,
                        &TypeSignature::PrincipalType,
                    );

                    // Push the offset and size to the data stack
                    builder.local_get(offset).i32_const(size);

                    // Call the host interface function, `contract_caller`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("contract_caller")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::TxSponsor => {
                    // Create a new local to hold the result on the call stack
                    let (offset, size);
                    (offset, size) = self.create_call_stack_local(
                        builder,
                        self.stack_pointer,
                        &TypeSignature::PrincipalType,
                    );

                    // Push the offset and size to the data stack
                    builder.local_get(offset).i32_const(size);

                    // Call the host interface function, `tx_sponsor`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("tx_sponsor")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::BlockHeight => {
                    // Call the host interface function, `block_height`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("block_height")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::BurnBlockHeight => {
                    // Call the host interface function, `burn_block_height`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("burn_block_height")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::NativeNone => {
                    add_placeholder_for_clarity_type(builder, ty);
                    true
                }
                NativeVariables::NativeTrue => {
                    builder.i32_const(1);
                    true
                }
                NativeVariables::NativeFalse => {
                    builder.i32_const(0);
                    true
                }
                NativeVariables::TotalLiquidMicroSTX => {
                    // Call the host interface function, `stx_liquid_supply`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("stx_liquid_supply")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::Regtest => {
                    // Call the host interface function, `is_in_regtest`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("is_in_regtest")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::Mainnet => {
                    // Call the host interface function, `is_in_mainnet`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("is_in_mainnet")
                            .expect("function not found"),
                    );
                    true
                }
                NativeVariables::ChainId => {
                    // Call the host interface function, `chain_id`
                    builder.call(
                        self.module
                            .funcs
                            .by_name("chain_id")
                            .expect("function not found"),
                    );
                    true
                }
            }
        } else {
            false
        }
    }

    /// If `name` is a constant, push its value onto the data stack.
    pub fn lookup_constant_variable<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        name: &str,
        ty: &TypeSignature,
    ) -> bool {
        if let Some(offset) = self.constants.get(name) {
            // Load the offset into a local variable
            let offset_local = self.module.locals.add(ValType::I32);
            builder.i32_const(*offset as i32).local_set(offset_local);

            // If `ty` is a value that stays in memory, we can just push the
            // offset and length to the stack.
            if is_in_memory_type(ty) {
                builder
                    .local_get(offset_local)
                    .i32_const(get_type_size(ty) as i32);
                true
            } else {
                // Otherwise, we need to load the value from memory.
                self.read_from_memory(builder, offset_local, 0, ty);
                true
            }
        } else {
            false
        }
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
        TypeSignature::SequenceType(_) => vec![
            ValType::I32, // offset
            ValType::I32, // length
        ],
        TypeSignature::BoolType => vec![ValType::I32],
        TypeSignature::PrincipalType => vec![
            ValType::I32, // offset
            ValType::I32, // length
        ],
        TypeSignature::OptionalType(inner_ty) => {
            let mut types = vec![ValType::I32];
            types.extend(clar2wasm_ty(inner_ty));
            types
        }
        TypeSignature::TupleType(inner_types) => {
            let mut types = vec![];
            for inner_type in inner_types.get_type_map().values() {
                types.extend(clar2wasm_ty(inner_type));
            }
            types
        }
        _ => unimplemented!("{:?}", ty),
    }
}

/// Drop a value of type `ty` from the data stack.
fn drop_value(builder: &mut InstrSeqBuilder, ty: &TypeSignature) {
    let wasm_types = clar2wasm_ty(ty);
    (0..wasm_types.len()).for_each(|_| {
        builder.drop();
    });
}

fn match_pairs_list(list: &[SymbolicExpression]) -> Option<Vec<TypedVar<'_>>> {
    let mut vars = Vec::new();
    for pair_list in list {
        let pair = pair_list.match_list()?;
        if pair.len() != 2 {
            return None;
        }
        let name = pair[0].match_atom()?;
        vars.push(TypedVar {
            name,
            type_expr: &pair[1],
            decl_span: pair[0].span.clone(),
        });
    }
    Some(vars)
}

fn match_tuple(
    expr: &SymbolicExpression,
) -> Option<HashMap<Option<&ClarityName>, &SymbolicExpression>> {
    if let Some(list) = expr.match_list() {
        if let Some((function_name, args)) = list.split_first() {
            if let Some(function_name) = function_name.match_atom() {
                if NativeFunctions::lookup_by_name_at_version(
                    function_name,
                    &clarity::vm::ClarityVersion::latest(),
                ) == Some(NativeFunctions::TupleCons)
                {
                    let mut tuple_map = HashMap::new();
                    for element in args {
                        let pair = element.match_list().unwrap_or_default();
                        if pair.len() != 2 {
                            return None;
                        }
                        tuple_map.insert(pair[0].match_atom(), &pair[1]);
                    }
                    return Some(tuple_map);
                }
            }
        }
    }
    None
}

fn match_pairs(expr: &SymbolicExpression) -> Option<HashMap<&ClarityName, &SymbolicExpression>> {
    let list = expr.match_list()?;
    let mut tuple_map = HashMap::new();
    for pair_list in list {
        let pair = pair_list.match_list()?;
        if pair.len() != 2 {
            return None;
        }
        tuple_map.insert(pair[0].match_atom()?, &pair[1]);
    }
    Some(tuple_map)
}

impl WasmGenerator {
    fn traverse_arithmetic<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> CResult<()> {
        let ty = self
            .get_expr_type(expr)
            .expect("arithmetic expression must be typed");
        let type_suffix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                self.error = Some(GeneratorError::InternalError(
                    "invalid type for arithmetic".to_string(),
                ));
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };
        let helper_func = match func {
            NativeFunctions::Add => self
                .module
                .funcs
                .by_name(&format!("add-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: add-{type_suffix}")),
            NativeFunctions::Subtract => self
                .module
                .funcs
                .by_name(&format!("sub-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: sub-{type_suffix}")),
            NativeFunctions::Multiply => self
                .module
                .funcs
                .by_name(&format!("mul-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: mul-{type_suffix}")),
            NativeFunctions::Divide => self
                .module
                .funcs
                .by_name(&format!("div-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: div-{type_suffix}")),
            NativeFunctions::Modulo => self
                .module
                .funcs
                .by_name(&format!("mod-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: mod-{type_suffix}")),
            NativeFunctions::Log2 => self
                .module
                .funcs
                .by_name(&format!("log2-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: log2-{type_suffix}")),
            NativeFunctions::Sqrti => self
                .module
                .funcs
                .by_name(&format!("sqrti-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: sqrti-{type_suffix}")),
            NativeFunctions::Power => self
                .module
                .funcs
                .by_name(&format!("pow-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: pow-{type_suffix}")),
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };

        // Start off with operand 0, then loop over the rest, calling the
        // helper function with a pair of operands, either operand 0 and 1, or
        // the result of the previous call and the next operand.
        // e.g. (+ 1 2 3 4) becomes (+ (+ (+ 1 2) 3) 4)
        self.traverse_expr(builder, &operands[0])?;
        for operand in operands.iter().skip(1) {
            self.traverse_expr(builder, operand)?;
            builder.call(helper_func);
        }

        Ok(())
    }

    fn traverse_bitwise<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        func: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> CResult<()> {
        let helper_func = match func {
            NativeFunctions::BitwiseAnd => self
                .module
                .funcs
                .by_name("bit-and")
                .unwrap_or_else(|| panic!("function not found: bit-and")),
            NativeFunctions::BitwiseOr => self
                .module
                .funcs
                .by_name("bit-or")
                .unwrap_or_else(|| panic!("function not found: bit-or")),
            NativeFunctions::BitwiseXor2 => self
                .module
                .funcs
                .by_name("bit-xor")
                .unwrap_or_else(|| panic!("function not found: bit-xor")),
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };

        // Start off with operand 0, then loop over the rest, calling the
        // helper function with a pair of operands, either operand 0 and 1, or
        // the result of the previous call and the next operand.
        // e.g. (+ 1 2 3 4) becomes (+ (+ (+ 1 2) 3) 4)
        self.traverse_expr(builder, &operands[0])?;
        for operand in operands.iter().skip(1) {
            self.traverse_expr(builder, operand)?;
            builder.call(helper_func);
        }

        Ok(())
    }

    fn visit_bit_shift<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        func: NativeFunctions,
        input: &SymbolicExpression,
        _shamt: &SymbolicExpression,
    ) -> CResult<()> {
        let ty = self
            .get_expr_type(input)
            .expect("bit shift operands must be typed");
        let type_suffix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            _ => {
                self.error = Some(GeneratorError::InternalError(
                    "invalid type for shift".to_string(),
                ));
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };
        let helper_func = match func {
            NativeFunctions::BitwiseLShift => self
                .module
                .funcs
                .by_name("bit-shift-left")
                .unwrap_or_else(|| panic!("function not found: bit-shift-left")),
            NativeFunctions::BitwiseRShift => self
                .module
                .funcs
                .by_name(&format!("bit-shift-right-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: bit-shift-right-{type_suffix}")),
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };
        builder.call(helper_func);

        Ok(())
    }

    fn visit_bitwise_not<'b>(&mut self, builder: &mut InstrSeqBuilder<'b>) -> CResult<()> {
        let helper_func = self
            .module
            .funcs
            .by_name("bit-not")
            .unwrap_or_else(|| panic!("function not found: bit-not"));
        builder.call(helper_func);
        Ok(())
    }

    fn visit_comparison<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        func: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> CResult<()> {
        let ty = self
            .get_expr_type(&operands[0])
            .expect("comparison operands must be typed");
        let type_suffix = match ty {
            TypeSignature::IntType => "int",
            TypeSignature::UIntType => "uint",
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(_))) => {
                "string-ascii"
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(_))) => {
                "string-utf8"
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(_)) => "buffer",
            _ => {
                self.error = Some(GeneratorError::InternalError(
                    "invalid type for comparison".to_string(),
                ));
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };
        let helper_func = match func {
            NativeFunctions::CmpLess => self
                .module
                .funcs
                .by_name(&format!("lt-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: lt-{type_suffix}")),
            NativeFunctions::CmpGreater => self
                .module
                .funcs
                .by_name(&format!("gt-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: gt-{type_suffix}")),
            NativeFunctions::CmpLeq => self
                .module
                .funcs
                .by_name(&format!("le-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: le-{type_suffix}")),
            NativeFunctions::CmpGeq => self
                .module
                .funcs
                .by_name(&format!("ge-{type_suffix}"))
                .unwrap_or_else(|| panic!("function not found: ge-{type_suffix}")),
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };
        builder.call(helper_func);

        Ok(())
    }

    fn visit_literal_value<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        value: &clarity::vm::Value,
    ) -> CResult<()> {
        match value {
            clarity::vm::Value::Int(i) => {
                builder.i64_const((i & 0xFFFFFFFFFFFFFFFF) as i64);
                builder.i64_const(((i >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                Ok(())
            }
            clarity::vm::Value::UInt(u) => {
                builder.i64_const((u & 0xFFFFFFFFFFFFFFFF) as i64);
                builder.i64_const(((u >> 64) & 0xFFFFFFFFFFFFFFFF) as i64);
                Ok(())
            }
            clarity::vm::Value::Sequence(SequenceData::String(s)) => {
                let (offset, len) = self.add_string_literal(s);
                builder.i32_const(offset as i32);
                builder.i32_const(len as i32);
                Ok(())
            }
            clarity::vm::Value::Principal(_)
            | clarity::vm::Value::Sequence(SequenceData::Buffer(_)) => {
                let (offset, len) = self.add_literal(value);
                builder.i32_const(offset as i32);
                builder.i32_const(len as i32);
                Ok(())
            }
            _ => {
                self.error = Some(GeneratorError::NotImplemented);
                Err(Box::new(GenericError) as Box<dyn std::error::Error>)
            }
        }
    }

    fn visit_atom<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        atom: &ClarityName,
    ) -> CResult<()> {
        let ty = match self.get_expr_type(expr) {
            Some(ty) => ty.clone(),
            None => {
                self.error = Some(GeneratorError::InternalError(
                    "atom expression must be typed".to_string(),
                ));
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };

        // Handle builtin variables
        let is_builtin: bool;
        is_builtin = self.lookup_reserved_variable(builder, atom.as_str(), &ty);
        if is_builtin {
            return Ok(());
        }

        // Handle constants
        let is_constant: bool;
        is_constant = self.lookup_constant_variable(builder, atom.as_str(), &ty);
        if is_constant {
            return Ok(());
        }

        let types = clar2wasm_ty(&ty);
        for n in 0..types.len() {
            let local = match self.locals.get(format!("{}.{}", atom.as_str(), n).as_str()) {
                Some(local) => *local,
                None => {
                    self.error = Some(GeneratorError::InternalError(format!(
                        "unable to find local for {}",
                        atom.as_str()
                    )));
                    return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
                }
            };
            builder.local_get(local);
        }

        Ok(())
    }

    fn traverse_define_private<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _parameters: Option<Vec<TypedVar<'_>>>,
        body: &SymbolicExpression,
    ) -> CResult<()> {
        if self
            .traverse_define_function(builder, name, body, FunctionKind::Private)
            .is_some()
        {
            Ok(())
        } else {
            Err(Box::new(GenericError) as Box<dyn std::error::Error>)
        }
    }

    fn traverse_define_read_only<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _parameters: Option<Vec<TypedVar<'_>>>,
        body: &SymbolicExpression,
    ) -> CResult<()> {
        if let Some(function_id) =
            self.traverse_define_function(builder, name, body, FunctionKind::ReadOnly)
        {
            self.module.exports.add(name.as_str(), function_id);
            Ok(())
        } else {
            Err(Box::new(GenericError) as Box<dyn std::error::Error>)
        }
    }

    fn traverse_define_public<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _parameters: Option<Vec<TypedVar<'_>>>,
        body: &SymbolicExpression,
    ) -> CResult<()> {
        if let Some(function_id) =
            self.traverse_define_function(builder, name, body, FunctionKind::Public)
        {
            self.module.exports.add(name.as_str(), function_id);
            Ok(())
        } else {
            Err(Box::new(GenericError) as Box<dyn std::error::Error>)
        }
    }

    fn traverse_define_data_var<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _data_type: &SymbolicExpression,
        initial: &SymbolicExpression,
    ) -> CResult<()> {
        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = self.add_identifier_string_literal(name);

        // The initial value can be placed on the top of the memory, since at
        // the top-level, we have not set up the call stack yet.
        let ty = self
            .get_expr_type(initial)
            .expect("initial value expression must be typed")
            .clone();
        let offset = self.module.locals.add(ValType::I32);
        builder
            .i32_const(self.literal_memory_end as i32)
            .local_set(offset);

        // Traverse the initial value for the data variable (result is on the
        // data stack)
        self.traverse_expr(builder, initial)?;

        // Write the initial value to the memory, to be read by the host.
        let size = self.write_to_memory(builder.borrow_mut(), offset, 0, &ty);

        // Increment the literal memory end
        // FIXME: These initial values do not need to be saved in the literal
        //        memory forever... we just need them once, when .top-level
        //        is called.
        self.literal_memory_end += size as u32;

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        // Push the offset onto the data stack
        builder.local_get(offset);

        // Push the size onto the data stack
        builder.i32_const(size);

        // Call the host interface function, `define_variable`
        builder.call(
            self.module
                .funcs
                .by_name("define_variable")
                .expect("function not found"),
        );
        Ok(())
    }

    fn traverse_define_ft<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        supply: Option<&SymbolicExpression>,
    ) -> CResult<()> {
        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = self.add_identifier_string_literal(name);

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        // Push the supply to the stack, as an optional uint
        // (first i32 indicates some/none)
        if let Some(supply) = supply {
            builder.i32_const(1);
            self.traverse_expr(builder, supply)?;
        } else {
            builder.i32_const(0).i64_const(0).i64_const(0);
        }

        builder.call(
            self.module
                .funcs
                .by_name("define_ft")
                .expect("function not found"),
        );
        Ok(())
    }

    fn traverse_define_nft<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _nft_type: &SymbolicExpression,
    ) -> CResult<()> {
        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = self.add_identifier_string_literal(name);

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        builder.call(
            self.module
                .funcs
                .by_name("define_nft")
                .expect("function not found"),
        );
        Ok(())
    }

    fn traverse_define_constant<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // If the initial value is a literal, then we can directly add it to
        // the literal memory.
        let offset = if let SymbolicExpressionType::LiteralValue(value) = &value.expr {
            let (offset, _len) = self.add_literal(value);
            offset
        } else {
            // If the initial expression is not a literal, then we need to
            // reserve the space for it, and then execute the expression and
            // write the result into the reserved space.
            let offset = self.literal_memory_end;
            let offset_local = self.module.locals.add(ValType::I32);
            builder.i32_const(offset as i32).local_set(offset_local);

            let ty = self
                .get_expr_type(value)
                .expect("constant value must be typed")
                .clone();

            let len = get_type_size(&ty);
            self.literal_memory_end += len;

            // Traverse the initial value expression.
            self.traverse_expr(builder, value)?;

            // Write the result (on the stack) to the memory
            self.write_to_memory(builder, offset_local, 0, &ty);

            offset
        };

        self.constants.insert(name.to_string(), offset);

        Ok(())
    }

    fn visit_define_map<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _key_type: &SymbolicExpression,
        _value_type: &SymbolicExpression,
    ) -> CResult<()> {
        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = self.add_identifier_string_literal(name);

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        builder.call(
            self.module
                .funcs
                .by_name("define_map")
                .expect("function not found"),
        );
        Ok(())
    }

    fn traverse_begin<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        statements: &[SymbolicExpression],
    ) -> CResult<()> {
        self.traverse_statement_list(builder, statements)
    }

    fn traverse_some<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // (some <val>) is represented by an i32 1, followed by the value
        builder.i32_const(1);
        self.traverse_expr(builder, value)?;
        Ok(())
    }

    fn traverse_ok<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // (ok <val>) is represented by an i32 1, followed by the ok value,
        // followed by a placeholder for the err value
        builder.i32_const(1);
        self.traverse_expr(builder, value)?;
        let ty = self
            .get_expr_type(expr)
            .expect("ok expression must be typed");
        if let TypeSignature::ResponseType(inner_types) = ty {
            let err_types = clar2wasm_ty(&inner_types.1);
            for err_type in err_types.iter() {
                add_placeholder_for_type(builder, *err_type);
            }
        } else {
            panic!("expected response type");
        }
        Ok(())
    }

    fn traverse_err<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // (err <val>) is represented by an i32 0, followed by a placeholder
        // for the ok value, followed by the err value
        builder.i32_const(0);
        let ty = self
            .get_expr_type(expr)
            .expect("err expression must be typed");
        if let TypeSignature::ResponseType(inner_types) = ty {
            let ok_types = clar2wasm_ty(&inner_types.0);
            for ok_type in ok_types.iter() {
                add_placeholder_for_type(builder, *ok_type);
            }
        } else {
            panic!("expected response type");
        }
        self.traverse_expr(builder, value)
    }

    fn visit_call_user_defined<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        _args: &[SymbolicExpression],
    ) -> CResult<()> {
        builder.call(
            self.module
                .funcs
                .by_name(name.as_str())
                .expect("function not found"),
        );
        Ok(())
    }

    fn traverse_concat<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        lhs: &SymbolicExpression,
        rhs: &SymbolicExpression,
    ) -> CResult<()> {
        // Create a new sequence to hold the result in the stack frame
        let ty = self
            .get_expr_type(expr)
            .expect("concat expression must be typed")
            .clone();
        let offset;
        (offset, _) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Traverse the lhs, leaving it on the data stack (offset, size)
        self.traverse_expr(builder, lhs)?;

        // Retrieve the memcpy function:
        // memcpy(src_offset, length, dst_offset)
        let memcpy = self
            .module
            .funcs
            .by_name("memcpy")
            .expect("function not found: memcpy");

        // Copy the lhs to the new sequence
        builder.local_get(offset).call(memcpy);

        // Save the new destination offset
        let end_offset = self.module.locals.add(ValType::I32);
        builder.local_set(end_offset);

        // Traverse the rhs, leaving it on the data stack (offset, size)
        self.traverse_expr(builder, rhs)?;

        // Copy the rhs to the new sequence
        builder.local_get(end_offset).call(memcpy);

        // Total size = end_offset - offset
        let size = self.module.locals.add(ValType::I32);
        builder
            .local_get(offset)
            .binop(BinaryOp::I32Sub)
            .local_set(size);

        // Return the new sequence (offset, size)
        builder.local_get(offset).local_get(size);

        Ok(())
    }

    fn visit_var_get<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
    ) -> CResult<()> {
        // Get the offset and length for this identifier in the literal memory
        let id_offset = *self
            .literal_memory_offet
            .get(name.as_str())
            .expect("variable not found: {name}");
        let id_length = name.len();

        // Create a new local to hold the result on the call stack
        let ty = self
            .get_expr_type(expr)
            .expect("var-get expression must be typed")
            .clone();
        let (offset, size);
        (offset, size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(size);

        // Call the host interface function, `get_variable`
        builder.call(
            self.module
                .funcs
                .by_name("get_variable")
                .expect("function not found"),
        );

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        self.read_from_memory(builder.borrow_mut(), offset, 0, &ty);

        Ok(())
    }

    fn visit_var_set<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // Get the offset and length for this identifier in the literal memory
        let id_offset = *self
            .literal_memory_offet
            .get(name.as_str())
            .expect("variable not found: {name}");
        let id_length = name.len();

        // Create space on the call stack to write the value
        let ty = self
            .get_expr_type(value)
            .expect("var-set value expression must be typed")
            .clone();
        let (offset, size);
        (offset, size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Write the value to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), offset, 0, &ty);

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(size);

        // Call the host interface function, `set_variable`
        builder.call(
            self.module
                .funcs
                .by_name("set_variable")
                .expect("function not found"),
        );

        // `var-set` always returns `true`
        builder.i32_const(1);

        Ok(())
    }

    fn traverse_list_cons<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        list: &[SymbolicExpression],
    ) -> CResult<()> {
        let ty = self
            .get_expr_type(expr)
            .expect("list expression must be typed")
            .clone();
        let (elem_ty, num_elem) =
            if let TypeSignature::SequenceType(SequenceSubtype::ListType(list_type)) = &ty {
                (list_type.get_list_item_type(), list_type.get_max_len())
            } else {
                panic!(
                    "Expected list type for list expression, but found: {:?}",
                    ty
                );
            };

        assert_eq!(num_elem as usize, list.len(), "list size mismatch");

        // Allocate space on the data stack for the entire list
        let (offset, size);
        (offset, size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Loop through the expressions in the list and store them onto the
        // data stack.
        let mut total_size = 0;
        for expr in list.iter() {
            self.traverse_expr(builder, expr)?;
            let elem_size = self.write_to_memory(builder.borrow_mut(), offset, total_size, elem_ty);
            total_size += elem_size as u32;
        }
        assert_eq!(total_size, size as u32, "list size mismatch");

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(size);

        Ok(())
    }

    fn traverse_fold<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        func: &ClarityName,
        sequence: &SymbolicExpression,
        initial: &SymbolicExpression,
    ) -> CResult<()> {
        // Fold takes an initial value, and a sequence, and applies a function
        // to the output of the previous call, or the initial value in the case
        // of the first call, and each element of the sequence.
        // ```
        // (fold - (list 2 4 6) 0)
        // ```
        // is equivalent to
        // ```
        // (- 6 (- 4 (- 2 0)))
        // ```

        // The result type must match the type of the initial value
        let result_clar_ty = self
            .get_expr_type(initial)
            .expect("fold's initial value expression must be typed");
        let result_ty = clar2wasm_ty(result_clar_ty);
        let loop_body_ty = InstrSeqType::new(&mut self.module.types, &[], &[]);

        // Get the type of the sequence
        let seq_ty = match self
            .get_expr_type(sequence)
            .expect("sequence expression must be typed")
        {
            TypeSignature::SequenceType(seq_ty) => seq_ty.clone(),
            _ => {
                self.error = Some(GeneratorError::InternalError(
                    "expected sequence type".to_string(),
                ));
                return Err(Box::new(GenericError) as Box<dyn std::error::Error>);
            }
        };

        let (seq_len, elem_ty) = match &seq_ty {
            SequenceSubtype::ListType(list_type) => {
                (list_type.get_max_len(), list_type.get_list_item_type())
            }
            _ => unimplemented!("Unsupported sequence type"),
        };

        // Evaluate the sequence, which will load it into the call stack,
        // leaving the offset and size on the data stack.
        self.traverse_expr(builder, sequence)?;

        // Drop the size, since we don't need it
        builder.drop();

        // Store the offset into a local
        let offset = self.module.locals.add(ValType::I32);
        builder.local_set(offset);

        let elem_size = get_type_size(elem_ty);

        // Store the end of the sequence into a local
        let end_offset = self.module.locals.add(ValType::I32);
        builder
            .local_get(offset)
            .i32_const((seq_len * elem_size) as i32)
            .binop(BinaryOp::I32Add)
            .local_set(end_offset);

        // Evaluate the initial value, so that its result is on the data stack
        self.traverse_expr(builder, initial)?;

        if seq_len == 0 {
            // If the sequence is empty, just return the initial value
            return Ok(());
        }

        // Define local(s) to hold the intermediate result, and initialize them
        // with the initial value. Not that we are looping in reverse order, to
        // pop values from the top of the stack.
        let mut result_locals = Vec::with_capacity(result_ty.len());
        for local_ty in result_ty.iter().rev() {
            let local = self.module.locals.add(*local_ty);
            result_locals.push(local);
            builder.local_set(local);
        }
        result_locals.reverse();

        // Define the body of a loop, to loop over the sequence and make the
        // function call.
        builder.loop_(loop_body_ty, |loop_| {
            let loop_id = loop_.id();

            // Load the element from the sequence
            let elem_size = self.read_from_memory(loop_, offset, 0, elem_ty);

            // Push the locals to the stack
            for result_local in result_locals.iter() {
                loop_.local_get(*result_local);
            }

            // Call the function
            loop_.call(
                self.module
                    .funcs
                    .by_name(func.as_str())
                    .expect("function not found"),
            );

            // Save the result into the locals (in reverse order as we pop)
            for result_local in result_locals.iter().rev() {
                loop_.local_set(*result_local);
            }

            // Increment the offset by the size of the element, leaving the
            // offset on the top of the stack
            loop_
                .local_get(offset)
                .i32_const(elem_size)
                .binop(BinaryOp::I32Add)
                .local_tee(offset);

            // Loop if we haven't reached the end of the sequence
            loop_
                .local_get(end_offset)
                .binop(BinaryOp::I32LtU)
                .br_if(loop_id);
        });

        // Push the locals to the stack
        for result_local in result_locals.iter() {
            builder.local_get(*result_local);
        }

        Ok(())
    }

    fn traverse_as_contract<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        inner: &SymbolicExpression,
    ) -> CResult<()> {
        // Call the host interface function, `enter_as_contract`
        builder.call(
            self.module
                .funcs
                .by_name("enter_as_contract")
                .expect("enter_as_contract not found"),
        );

        // Traverse the inner expression
        self.traverse_expr(builder, inner)?;

        // Call the host interface function, `exit_as_contract`
        builder.call(
            self.module
                .funcs
                .by_name("exit_as_contract")
                .expect("exit_as_contract not found"),
        );

        Ok(())
    }

    fn visit_stx_get_balance<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _owner: &SymbolicExpression,
    ) -> CResult<()> {
        // Owner is on the stack, so just call the host interface function,
        // `stx_get_balance`
        builder.call(
            self.module
                .funcs
                .by_name("stx_get_balance")
                .expect("stx_get_balance not found"),
        );
        Ok(())
    }

    fn visit_stx_get_account<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _owner: &SymbolicExpression,
    ) -> CResult<()> {
        // Owner is on the stack, so just call the host interface function,
        // `stx_get_account`
        builder.call(
            self.module
                .funcs
                .by_name("stx_account")
                .expect("stx_account not found"),
        );
        Ok(())
    }

    fn visit_stx_burn<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _amount: &SymbolicExpression,
        _sender: &SymbolicExpression,
    ) -> CResult<()> {
        // Amount and sender are on the stack, so just call the host interface
        // function, `stx_burn`
        builder.call(
            self.module
                .funcs
                .by_name("stx_burn")
                .expect("stx_burn not found"),
        );
        Ok(())
    }

    fn visit_stx_transfer<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _amount: &SymbolicExpression,
        _sender: &SymbolicExpression,
        _recipient: &SymbolicExpression,
        _memo: Option<&SymbolicExpression>,
    ) -> CResult<()> {
        // Amount, sender, and recipient are on the stack. If memo is none, we
        // need to add a placeholder to the stack, then we can call the host
        // interface function, `stx_transfer`
        if _memo.is_none() {
            builder.i32_const(0).i32_const(0);
        }
        builder.call(
            self.module
                .funcs
                .by_name("stx_transfer")
                .expect("stx_transfer not found"),
        );
        Ok(())
    }

    fn visit_ft_get_supply<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
    ) -> CResult<()> {
        // Push the token name onto the stack, then call the host interface
        // function `ft_get_supply`
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        builder.call(
            self.module
                .funcs
                .by_name("ft_get_supply")
                .expect("ft_get_supply not found"),
        );

        Ok(())
    }

    fn traverse_ft_get_balance<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        owner: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the owner onto the stack
        self.traverse_expr(builder, owner)?;

        // Call the host interface function `ft_get_balance`
        builder.call(
            self.module
                .funcs
                .by_name("ft_get_balance")
                .expect("ft_get_balance not found"),
        );

        Ok(())
    }

    fn traverse_ft_burn<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the amount and sender onto the stack
        self.traverse_expr(builder, amount)?;
        self.traverse_expr(builder, sender)?;

        // Call the host interface function `ft_burn`
        builder.call(
            self.module
                .funcs
                .by_name("ft_burn")
                .expect("ft_burn not found"),
        );

        Ok(())
    }

    fn traverse_ft_mint<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        amount: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the amount and recipient onto the stack
        self.traverse_expr(builder, amount)?;
        self.traverse_expr(builder, recipient)?;

        // Call the host interface function `ft_mint`
        builder.call(
            self.module
                .funcs
                .by_name("ft_mint")
                .expect("ft_mint not found"),
        );

        Ok(())
    }

    fn traverse_ft_transfer<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the amount, sender, and recipient onto the stack
        self.traverse_expr(builder, amount)?;
        self.traverse_expr(builder, sender)?;
        self.traverse_expr(builder, recipient)?;

        // Call the host interface function `ft_transfer`
        builder.call(
            self.module
                .funcs
                .by_name("ft_transfer")
                .expect("ft_transfer not found"),
        );

        Ok(())
    }

    fn traverse_nft_get_owner<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the identifier onto the stack
        self.traverse_expr(builder, identifier)?;

        let identifier_ty = self
            .get_expr_type(identifier)
            .expect("NFT identifier must be typed")
            .clone();

        // Allocate space on the stack for the identifier
        let id_offset;
        let id_size;
        (id_offset, id_size) =
            self.create_call_stack_local(builder, self.stack_pointer, &identifier_ty);

        // Write the identifier to the stack (since the host needs to handle generic types)
        self.write_to_memory(builder, id_offset, 0, &identifier_ty);

        // Push the offset and size to the data stack
        builder.local_get(id_offset).i32_const(id_size);

        // Reserve stack space for the return value, a principal
        let return_offset;
        let return_size;
        (return_offset, return_size) = self.create_call_stack_local(
            builder,
            self.stack_pointer,
            &TypeSignature::PrincipalType,
        );

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function `nft_get_owner`
        builder.call(
            self.module
                .funcs
                .by_name("nft_get_owner")
                .expect("nft_get_owner not found"),
        );

        Ok(())
    }

    fn traverse_nft_burn<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
        sender: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the identifier onto the stack
        self.traverse_expr(builder, identifier)?;

        let identifier_ty = self
            .get_expr_type(identifier)
            .expect("NFT identifier must be typed")
            .clone();

        // Allocate space on the stack for the identifier
        let id_offset;
        let id_size;
        (id_offset, id_size) =
            self.create_call_stack_local(builder, self.stack_pointer, &identifier_ty);

        // Write the identifier to the stack (since the host needs to handle generic types)
        self.write_to_memory(builder, id_offset, 0, &identifier_ty);

        // Push the offset and size to the data stack
        builder.local_get(id_offset).i32_const(id_size);

        // Push the sender onto the stack
        self.traverse_expr(builder, sender)?;

        // Call the host interface function `nft_burn`
        builder.call(
            self.module
                .funcs
                .by_name("nft_burn")
                .expect("nft_burn not found"),
        );

        Ok(())
    }

    fn traverse_nft_mint<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the identifier onto the stack
        self.traverse_expr(builder, identifier)?;

        let identifier_ty = self
            .get_expr_type(identifier)
            .expect("NFT identifier must be typed")
            .clone();

        // Allocate space on the stack for the identifier
        let id_offset;
        let id_size;
        (id_offset, id_size) =
            self.create_call_stack_local(builder, self.stack_pointer, &identifier_ty);

        // Write the identifier to the stack (since the host needs to handle generic types)
        self.write_to_memory(builder, id_offset, 0, &identifier_ty);

        // Push the offset and size to the data stack
        builder.local_get(id_offset).i32_const(id_size);

        // Push the recipient onto the stack
        self.traverse_expr(builder, recipient)?;

        // Call the host interface function `nft_mint`
        builder.call(
            self.module
                .funcs
                .by_name("nft_mint")
                .expect("nft_mint not found"),
        );

        Ok(())
    }

    fn traverse_nft_transfer<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
        sender: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the token name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(token);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the identifier onto the stack
        self.traverse_expr(builder, identifier)?;

        let identifier_ty = self
            .get_expr_type(identifier)
            .expect("NFT identifier must be typed")
            .clone();

        // Allocate space on the stack for the identifier
        let id_offset;
        let id_size;
        (id_offset, id_size) =
            self.create_call_stack_local(builder, self.stack_pointer, &identifier_ty);

        // Write the identifier to the stack (since the host needs to handle generic types)
        self.write_to_memory(builder, id_offset, 0, &identifier_ty);

        // Push the offset and size to the data stack
        builder.local_get(id_offset).i32_const(id_size);

        // Push the sender onto the stack
        self.traverse_expr(builder, sender)?;

        // Push the recipient onto the stack
        self.traverse_expr(builder, recipient)?;

        // Call the host interface function `nft_transfer`
        builder.call(
            self.module
                .funcs
                .by_name("nft_transfer")
                .expect("nft_transfer not found"),
        );

        Ok(())
    }

    fn visit_unwrap_panic<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        // There must be either an `optional` or a `response` on the top of the
        // stack. Both use an i32 indicator, where 0 means `none` or `err`. In
        // both cases, if this indicator is a 0, then we need to early exit.

        // Get the type of the input expression
        let input_ty = self
            .get_expr_type(input)
            .expect("try input expression must be typed")
            .clone();

        match &input_ty {
            TypeSignature::OptionalType(val_ty) => {
                // For the optional case, e.g. `(unwrap-panic (some 1))`, the stack
                // will look like:
                // 1 -- some value
                // 1 -- indicator
                // We need to get to the indicator, so we can pop the some value and
                // store it in a local, then check the indicator. If it's 0, we need to
                // trigger a runtime error. If it's a 1, we just push the some value
                // back onto the stack and continue execution.

                // Save the value in locals
                let wasm_types = clar2wasm_ty(val_ty);
                let mut val_locals = Vec::with_capacity(wasm_types.len());
                for local_ty in wasm_types.iter().rev() {
                    let local = self.module.locals.add(*local_ty);
                    val_locals.push(local);
                    builder.local_set(local);
                }

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    InstrSeqType::new(&mut self.module.types, &[], &[]),
                    |then| {
                        then.i32_const(Trap::Panic as i32).call(
                            self.module
                                .funcs
                                .by_name("runtime-error")
                                .expect("runtime_error not found"),
                        );
                    },
                    |_| {},
                );

                // Otherwise, push the value back onto the stack
                for &val_local in val_locals.iter().rev() {
                    builder.local_get(val_local);
                }

                Ok(())
            }
            TypeSignature::ResponseType(inner_types) => {
                // Ex. `(unwrap-panic (ok 1))`, where the value type is
                // `(response uint uint)`, the stack will look like:
                // 0 -- err value
                // 1 -- ok value
                // 1 -- indicator
                // We need to get to the indicator, so we can drop the err value, since
                // that is not needed, then we can pop the ok value and store them in a
                // local. Now we can check the indicator. If it's 0, we need to trigger
                // a runtime error. If it's a 1, we just push the ok value back onto
                // the stack and continue execution.

                let (ok_ty, err_ty) = &**inner_types;

                // Drop the err value
                drop_value(builder, err_ty);

                // Save the ok value in locals
                let ok_wasm_types = clar2wasm_ty(ok_ty);
                let mut ok_val_locals = Vec::with_capacity(ok_wasm_types.len());
                for local_ty in ok_wasm_types.iter().rev() {
                    let local = self.module.locals.add(*local_ty);
                    ok_val_locals.push(local);
                    builder.local_set(local);
                }

                // If the indicator is 0, throw a runtime error
                builder.unop(UnaryOp::I32Eqz).if_else(
                    InstrSeqType::new(&mut self.module.types, &[], &[]),
                    |then| {
                        then.i32_const(Trap::Panic as i32).call(
                            self.module
                                .funcs
                                .by_name("runtime-error")
                                .expect("runtime_error not found"),
                        );
                    },
                    |_| {},
                );

                // Otherwise, push the value back onto the stack
                for &val_local in ok_val_locals.iter().rev() {
                    builder.local_get(val_local);
                }

                Ok(())
            }
            _ => Err(Box::new(GenericError) as Box<dyn std::error::Error>),
        }
    }

    fn traverse_map_get<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
        key: &SymbolicExpression,
    ) -> CResult<()> {
        // Get the offset and length for this identifier in the literal memory
        let id_offset = *self
            .literal_memory_offet
            .get(name.as_str())
            .expect("map not found: {name}");
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = self
            .get_expr_type(key)
            .expect("map-set value expression must be typed")
            .clone();
        let (key_offset, key_size);
        (key_offset, key_size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the key to the data stack
        self.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), key_offset, 0, &ty);

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Create a new local to hold the result on the call stack
        let ty = self
            .get_expr_type(expr)
            .expect("map-get? expression must be typed")
            .clone();
        let (return_offset, return_size);
        (return_offset, return_size) =
            self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the return value offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host-interface function, `map_get`
        builder.call(
            self.module
                .funcs
                .by_name("map_get")
                .expect("map_get not found"),
        );

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        self.read_from_memory(builder.borrow_mut(), return_offset, 0, &ty);

        Ok(())
    }

    fn traverse_map_set<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        key: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // Get the offset and length for this identifier in the literal memory
        let id_offset = *self
            .literal_memory_offet
            .get(name.as_str())
            .expect("map not found: {name}");
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = self
            .get_expr_type(key)
            .expect("map-set value expression must be typed")
            .clone();
        let (key_offset, key_size);
        (key_offset, key_size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the key to the data stack
        self.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), key_offset, 0, &ty);

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Create space on the call stack to write the value
        let ty = self
            .get_expr_type(value)
            .expect("map-set value expression must be typed")
            .clone();
        let (val_offset, val_size);
        (val_offset, val_size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the value to the data stack
        self.traverse_expr(builder, value)?;

        // Write the value to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), val_offset, 0, &ty);

        // Push the value offset and size to the data stack
        builder.local_get(val_offset).i32_const(val_size);

        // Call the host interface function, `map_set`
        builder.call(
            self.module
                .funcs
                .by_name("map_set")
                .expect("map_set not found"),
        );

        Ok(())
    }

    fn traverse_map_insert<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        key: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        // Get the offset and length for this identifier in the literal memory
        let id_offset = *self
            .literal_memory_offet
            .get(name.as_str())
            .expect("map not found: {name}");
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = self
            .get_expr_type(key)
            .expect("map-set value expression must be typed")
            .clone();
        let (key_offset, key_size);
        (key_offset, key_size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the key to the data stack
        self.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), key_offset, 0, &ty);

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Create space on the call stack to write the value
        let ty = self
            .get_expr_type(value)
            .expect("map-set value expression must be typed")
            .clone();
        let (val_offset, val_size);
        (val_offset, val_size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the value to the data stack
        self.traverse_expr(builder, value)?;

        // Write the value to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), val_offset, 0, &ty);

        // Push the value offset and size to the data stack
        builder.local_get(val_offset).i32_const(val_size);

        // Call the host interface function, `map_insert`
        builder.call(
            self.module
                .funcs
                .by_name("map_insert")
                .expect("map_insert not found"),
        );

        Ok(())
    }

    fn traverse_map_delete<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        name: &ClarityName,
        key: &SymbolicExpression,
    ) -> CResult<()> {
        // Get the offset and length for this identifier in the literal memory
        let id_offset = *self
            .literal_memory_offet
            .get(name.as_str())
            .expect("map not found: {name}");
        let id_length = name.len();

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Create space on the call stack to write the key
        let ty = self
            .get_expr_type(key)
            .expect("map-set value expression must be typed")
            .clone();
        let (key_offset, key_size);
        (key_offset, key_size) = self.create_call_stack_local(builder, self.stack_pointer, &ty);

        // Push the key to the data stack
        self.traverse_expr(builder, key)?;

        // Write the key to the memory (it's already on the data stack)
        self.write_to_memory(builder.borrow_mut(), key_offset, 0, &ty);

        // Push the key offset and size to the data stack
        builder.local_get(key_offset).i32_const(key_size);

        // Call the host interface function, `map_delete`
        builder.call(
            self.module
                .funcs
                .by_name("map_delete")
                .expect("map_delete not found"),
        );

        Ok(())
    }

    fn traverse_get_block_info<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        prop_name: &ClarityName,
        block: &SymbolicExpression,
    ) -> CResult<()> {
        // Push the property name onto the stack
        let (id_offset, id_length) = self.add_identifier_string_literal(prop_name);
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the block number onto the stack
        self.traverse_expr(builder, block)?;

        // Reserve space on the stack for the return value
        let return_ty = self
            .get_expr_type(expr)
            .expect("get-block-info? expression must be typed")
            .clone();

        let (return_offset, return_size);
        (return_offset, return_size) =
            self.create_call_stack_local(builder, self.stack_pointer, &return_ty);

        // Push the offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `get_block_info`
        builder.call(
            self.module
                .funcs
                .by_name("get_block_info")
                .expect("get_block_info not found"),
        );

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        self.read_from_memory(builder.borrow_mut(), return_offset, 0, &return_ty);

        Ok(())
    }

    fn traverse_call_user_defined<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
        args: &[SymbolicExpression],
    ) -> CResult<()> {
        for arg in args.iter() {
            self.traverse_expr(builder, arg)?;
        }
        self.visit_call_user_defined(builder, expr, name, args)
    }

    fn traverse_bit_shift<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: NativeFunctions,
        input: &SymbolicExpression,
        shamt: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.traverse_expr(builder, shamt)?;
        self.visit_bit_shift(builder, expr, func, input, shamt)
    }

    fn traverse_replace_at<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        sequence: &SymbolicExpression,
        index: &SymbolicExpression,
        element: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, sequence)?;
        self.traverse_expr(builder, index)?;
        self.traverse_expr(builder, element)?;
        self.visit_replace_at(builder, expr, sequence, element, index)
    }

    fn traverse_bitwise_not<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_bitwise_not(builder)
    }

    fn visit_field<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _field: &TraitIdentifier,
    ) -> CResult<()> {
        Ok(())
    }

    fn visit_trait_reference<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _name: &ClarityName,
        _trait_def: &TraitDefinition,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_define_map<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
        key_type: &SymbolicExpression,
        value_type: &SymbolicExpression,
    ) -> CResult<()> {
        self.visit_define_map(builder, expr, name, key_type, value_type)
    }

    fn traverse_define_trait<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
        functions: &[SymbolicExpression],
    ) -> CResult<()> {
        self.visit_define_trait(builder, expr, name, functions)
    }

    fn visit_define_trait<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _name: &ClarityName,
        _functions: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_use_trait<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
        trait_identifier: &TraitIdentifier,
    ) -> CResult<()> {
        self.visit_use_trait(builder, expr, name, trait_identifier)
    }

    fn visit_use_trait<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _name: &ClarityName,
        _trait_identifier: &TraitIdentifier,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_impl_trait<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        trait_identifier: &TraitIdentifier,
    ) -> CResult<()> {
        self.visit_impl_trait(builder, expr, trait_identifier)
    }

    fn visit_impl_trait<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _trait_identifier: &TraitIdentifier,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_binary_bitwise<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: NativeFunctions,
        lhs: &SymbolicExpression,
        rhs: &SymbolicExpression,
    ) -> CResult<()> {
        for operand in &[lhs, rhs] {
            self.traverse_expr(builder, operand)?;
        }
        self.visit_binary_bitwise(builder, expr, func, lhs, rhs)
    }

    fn visit_binary_bitwise<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _func: NativeFunctions,
        _lhs: &SymbolicExpression,
        _rhs: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_comparison<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> CResult<()> {
        for operand in operands {
            self.traverse_expr(builder, operand)?;
        }
        self.visit_comparison(builder, expr, func, operands)
    }

    fn traverse_lazy_logical<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        function: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> CResult<()> {
        for operand in operands {
            self.traverse_expr(builder, operand)?;
        }
        self.visit_lazy_logical(builder, expr, function, operands)
    }

    fn visit_lazy_logical<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _function: NativeFunctions,
        _operands: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_logical<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        function: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> CResult<()> {
        for operand in operands {
            self.traverse_expr(builder, operand)?;
        }
        self.visit_logical(builder, expr, function, operands)
    }

    fn visit_logical<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _function: NativeFunctions,
        _operands: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_int_cast<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_int_cast(builder, expr, input)
    }

    fn visit_int_cast<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_if<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        cond: &SymbolicExpression,
        then_expr: &SymbolicExpression,
        else_expr: &SymbolicExpression,
    ) -> CResult<()> {
        for &expr in &[cond, then_expr, else_expr] {
            self.traverse_expr(builder, expr)?;
        }
        self.visit_if(builder, expr, cond, then_expr, else_expr)
    }

    fn visit_if<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _cond: &SymbolicExpression,
        _then_expr: &SymbolicExpression,
        _else_expr: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_var_get<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
    ) -> CResult<()> {
        self.visit_var_get(builder, expr, name)
    }

    fn traverse_var_set<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &ClarityName,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_var_set(builder, expr, name, value)
    }

    fn traverse_tuple<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        values: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
    ) -> CResult<()> {
        for val in values.values() {
            self.traverse_expr(builder, val)?;
        }
        self.visit_tuple(builder, expr, values)
    }

    fn visit_tuple<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _values: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_get<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        key: &ClarityName,
        tuple: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, tuple)?;
        self.visit_get(builder, expr, key, tuple)
    }

    fn visit_get<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _key: &ClarityName,
        _tuple: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_merge<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        tuple1: &SymbolicExpression,
        tuple2: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, tuple1)?;
        self.traverse_expr(builder, tuple2)?;
        self.visit_merge(builder, expr, tuple1, tuple2)
    }

    fn visit_merge<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _tuple1: &SymbolicExpression,
        _tuple2: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_hash<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: NativeFunctions,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_hash(builder, expr, func, value)
    }

    fn visit_hash<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _func: NativeFunctions,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_secp256k1_recover<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        hash: &SymbolicExpression,
        signature: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, hash)?;
        self.traverse_expr(builder, signature)?;
        self.visit_secp256k1_recover(builder, expr, hash, signature)
    }

    fn visit_secp256k1_recover<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _hash: &SymbolicExpression,
        _signature: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_secp256k1_verify<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        hash: &SymbolicExpression,
        signature: &SymbolicExpression,
        public_key: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, hash)?;
        self.traverse_expr(builder, signature)?;
        self.visit_secp256k1_verify(builder, expr, hash, signature, public_key)
    }

    fn visit_secp256k1_verify<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _hash: &SymbolicExpression,
        _signature: &SymbolicExpression,
        _public_key: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_print<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_print(builder, expr, value)
    }

    fn visit_print<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_static_contract_call<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        contract_identifier: &QualifiedContractIdentifier,
        function_name: &ClarityName,
        args: &[SymbolicExpression],
    ) -> CResult<()> {
        for arg in args.iter() {
            self.traverse_expr(builder, arg)?;
        }
        self.visit_static_contract_call(builder, expr, contract_identifier, function_name, args)
    }

    fn visit_static_contract_call<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _contract_identifier: &QualifiedContractIdentifier,
        _function_name: &ClarityName,
        _args: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_dynamic_contract_call<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        trait_ref: &SymbolicExpression,
        function_name: &ClarityName,
        args: &[SymbolicExpression],
    ) -> CResult<()> {
        self.traverse_expr(builder, trait_ref)?;
        for arg in args.iter() {
            self.traverse_expr(builder, arg)?;
        }
        self.visit_dynamic_contract_call(builder, expr, trait_ref, function_name, args)
    }

    fn visit_dynamic_contract_call<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _trait_ref: &SymbolicExpression,
        _function_name: &ClarityName,
        _args: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_contract_of<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        name: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, name)?;
        self.visit_contract_of(builder, expr, name)
    }

    fn visit_contract_of<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _name: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_principal_of<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        public_key: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, public_key)?;
        self.visit_principal_of(builder, expr, public_key)
    }

    fn visit_principal_of<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _public_key: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_at_block<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        block: &SymbolicExpression,
        inner: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, block)?;
        self.traverse_expr(builder, inner)?;
        self.visit_at_block(builder, expr, block, inner)
    }

    fn visit_at_block<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _block: &SymbolicExpression,
        _inner: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_default_to<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        default: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, default)?;
        self.traverse_expr(builder, value)?;
        self.visit_default_to(builder, expr, default, value)
    }

    fn visit_default_to<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _default: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_unwrap<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
        throws: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.traverse_expr(builder, throws)?;
        self.visit_unwrap(builder, expr, input, throws)
    }

    fn visit_unwrap<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
        _throws: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_unwrap_err<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
        throws: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.traverse_expr(builder, throws)?;
        self.visit_unwrap_err(builder, expr, input, throws)
    }

    fn visit_unwrap_err<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
        _throws: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_is_ok<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_is_ok(builder, expr, value)
    }

    fn visit_is_ok<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_is_none<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_is_none(builder, expr, value)
    }

    fn visit_is_none<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_is_err<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_is_err(builder, expr, value)
    }

    fn visit_is_err<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_is_some<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_is_some(builder, expr, value)
    }

    fn visit_is_some<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_filter<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: &ClarityName,
        sequence: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, sequence)?;
        self.visit_filter(builder, expr, func, sequence)
    }

    fn visit_filter<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _func: &ClarityName,
        _sequence: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_unwrap_panic<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_unwrap_panic(builder, expr, input)
    }

    fn traverse_match_option<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
        some_name: &ClarityName,
        some_branch: &SymbolicExpression,
        none_branch: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.traverse_expr(builder, some_branch)?;
        self.traverse_expr(builder, none_branch)?;
        self.visit_match_option(builder, expr, input, some_name, some_branch, none_branch)
    }

    fn visit_match_option<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
        _some_name: &ClarityName,
        _some_branch: &SymbolicExpression,
        _none_branch: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn traverse_match_response<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
        ok_name: &ClarityName,
        ok_branch: &SymbolicExpression,
        err_name: &ClarityName,
        err_branch: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.traverse_expr(builder, ok_branch)?;
        self.traverse_expr(builder, err_branch)?;
        self.visit_match_response(
            builder, expr, input, ok_name, ok_branch, err_name, err_branch,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn visit_match_response<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
        _ok_name: &ClarityName,
        _ok_branch: &SymbolicExpression,
        _err_name: &ClarityName,
        _err_branch: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_try<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_try(builder, expr, input)
    }

    fn visit_try<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_asserts<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        cond: &SymbolicExpression,
        thrown: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, cond)?;
        self.traverse_expr(builder, thrown)?;
        self.visit_asserts(builder, expr, cond, thrown)
    }

    fn visit_asserts<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _cond: &SymbolicExpression,
        _thrown: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_stx_burn<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, amount)?;
        self.traverse_expr(builder, sender)?;
        self.visit_stx_burn(builder, expr, amount, sender)
    }

    fn traverse_stx_transfer<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
        recipient: &SymbolicExpression,
        memo: Option<&SymbolicExpression>,
    ) -> CResult<()> {
        self.traverse_expr(builder, amount)?;
        self.traverse_expr(builder, sender)?;
        self.traverse_expr(builder, recipient)?;
        if let Some(memo) = memo {
            self.traverse_expr(builder, memo)?;
        }
        self.visit_stx_transfer(builder, expr, amount, sender, recipient, memo)
    }

    fn traverse_stx_get_balance<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        owner: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, owner)?;
        self.visit_stx_get_balance(builder, expr, owner)
    }

    fn traverse_ft_get_supply<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        token: &ClarityName,
    ) -> CResult<()> {
        self.visit_ft_get_supply(builder, expr, token)
    }

    fn traverse_let<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        bindings: &HashMap<&ClarityName, &SymbolicExpression>,
        body: &[SymbolicExpression],
    ) -> CResult<()> {
        for val in bindings.values() {
            self.traverse_expr(builder, val)?;
        }
        for expr in body {
            self.traverse_expr(builder, expr)?;
        }
        self.visit_let(builder, expr, bindings, body)
    }

    fn visit_let<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _bindings: &HashMap<&ClarityName, &SymbolicExpression>,
        _body: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_map<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        func: &ClarityName,
        sequences: &[SymbolicExpression],
    ) -> CResult<()> {
        for sequence in sequences {
            self.traverse_expr(builder, sequence)?;
        }
        self.visit_map(builder, expr, func, sequences)
    }

    fn visit_map<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _func: &ClarityName,
        _sequences: &[SymbolicExpression],
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_append<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        list: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, list)?;
        self.traverse_expr(builder, value)?;
        self.visit_append(builder, expr, list, value)
    }

    fn visit_append<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _list: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_as_max_len<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        sequence: &SymbolicExpression,
        length: u128,
    ) -> CResult<()> {
        self.traverse_expr(builder, sequence)?;
        self.visit_as_max_len(builder, expr, sequence, length)
    }

    fn visit_as_max_len<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _sequence: &SymbolicExpression,
        _length: u128,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_len<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        sequence: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, sequence)?;
        self.visit_len(builder, expr, sequence)
    }

    fn visit_len<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _sequence: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_element_at<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        sequence: &SymbolicExpression,
        index: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, sequence)?;
        self.traverse_expr(builder, index)?;
        self.visit_element_at(builder, expr, sequence, index)
    }

    fn visit_element_at<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _sequence: &SymbolicExpression,
        _index: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_index_of<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        sequence: &SymbolicExpression,
        item: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, sequence)?;
        self.traverse_expr(builder, item)?;
        self.visit_element_at(builder, expr, sequence, item)
    }

    fn traverse_buff_cast<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_buff_cast(builder, expr, input)
    }

    fn visit_buff_cast<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_is_standard<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        value: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, value)?;
        self.visit_is_standard(builder, expr, value)
    }

    fn visit_is_standard<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _value: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_principal_destruct<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        principal: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, principal)?;
        self.visit_principal_destruct(builder, expr, principal)
    }

    fn visit_principal_destruct<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _principal: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_principal_construct<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        buff1: &SymbolicExpression,
        buff20: &SymbolicExpression,
        contract: Option<&SymbolicExpression>,
    ) -> CResult<()> {
        self.traverse_expr(builder, buff1)?;
        self.traverse_expr(builder, buff20)?;
        if let Some(contract) = contract {
            self.traverse_expr(builder, contract)?;
        }
        self.visit_principal_construct(builder, expr, buff1, buff20, contract)
    }

    fn visit_principal_construct<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _buff1: &SymbolicExpression,
        _buff20: &SymbolicExpression,
        _contract: Option<&SymbolicExpression>,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_string_to_int<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_string_to_int(builder, expr, input)
    }

    fn visit_string_to_int<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_int_to_string<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_int_to_string(builder, expr, input)
    }

    fn visit_int_to_string<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_stx_get_account<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        owner: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, owner)?;
        self.visit_stx_get_account(builder, expr, owner)
    }

    fn traverse_slice<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        seq: &SymbolicExpression,
        left: &SymbolicExpression,
        right: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, seq)?;
        self.traverse_expr(builder, left)?;
        self.traverse_expr(builder, right)?;
        self.visit_slice(builder, expr, seq, left, right)
    }

    fn visit_slice<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _seq: &SymbolicExpression,
        _left: &SymbolicExpression,
        _right: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_get_burn_block_info<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        prop_name: &ClarityName,
        block: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, block)?;
        self.visit_get_burn_block_info(builder, expr, prop_name, block)
    }

    fn visit_get_burn_block_info<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _prop_name: &ClarityName,
        _block: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_to_consensus_buff<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, input)?;
        self.visit_to_consensus_buff(builder, expr, input)
    }

    fn visit_to_consensus_buff<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn traverse_from_consensus_buff<'b>(
        &mut self,
        builder: &mut InstrSeqBuilder<'b>,
        expr: &SymbolicExpression,
        type_expr: &SymbolicExpression,
        input: &SymbolicExpression,
    ) -> CResult<()> {
        self.traverse_expr(builder, type_expr)?;
        self.traverse_expr(builder, input)?;
        self.visit_from_consensus_buff(builder, expr, type_expr, input)
    }

    fn visit_from_consensus_buff<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _type_expr: &SymbolicExpression,
        _input: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }

    fn visit_replace_at<'b>(
        &mut self,
        _builder: &mut InstrSeqBuilder<'b>,
        _expr: &SymbolicExpression,
        _sequence: &SymbolicExpression,
        _index: &SymbolicExpression,
        _element: &SymbolicExpression,
    ) -> CResult<()> {
        Ok(())
    }
}
