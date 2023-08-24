// This file is copied from [Clarinet](https://github.com/hirosystems/clarinet),
// which is licensed under the GPPLv3 license.

use clarity::vm::functions::define::DefineFunctions;
use clarity::vm::functions::NativeFunctions;
use clarity::vm::representations::SymbolicExpressionType::*;
use clarity::vm::representations::{Span, TraitDefinition};
use clarity::vm::types::{PrincipalData, QualifiedContractIdentifier, TraitIdentifier, Value};
use clarity::vm::{ClarityName, ClarityVersion, SymbolicExpression, SymbolicExpressionType};
use std::collections::HashMap;
use walrus::InstrSeqBuilder;

#[derive(Clone)]
pub struct TypedVar<'c> {
    pub name: &'c ClarityName,
    pub type_expr: &'c SymbolicExpression,
    pub decl_span: Span,
}

lazy_static! {
    // Since the AST Visitor may be used before other checks have been performed,
    // we may need a default value for some expressions. This can be used for a
    // missing `ClarityName`.
    static ref DEFAULT_NAME: ClarityName = ClarityName::from("placeholder__");
    static ref DEFAULT_EXPR: SymbolicExpression = SymbolicExpression::atom(DEFAULT_NAME.clone());
}

/// The ASTVisitor trait specifies the interfaces needed to build a visitor
/// to walk a Clarity abstract syntax tree (AST). All methods have default
/// implementations so that any left undefined in an implementation will
/// perform a standard walk through the AST, ensuring that all sub-expressions
/// are visited as appropriate. If a `traverse_*` method is implemented, then
/// the implementation is responsible for traversing the sub-expressions.
///
/// Traversal is post-order, so the sub-expressions are visited before the
/// parent is visited. To walk through an example, if we visit the AST for the
/// Clarity expression `(+ a 1)`, we would hit the following methods in order:
/// 1. `traverse_expr`: `(+ a 1)`
/// 2. `traverse_list`: `(+ a 1)`
/// 3. `traverse_arithmetic`: `(+ a 1)`
/// 4. `traverse_expr`: `a`
/// 5. `visit_atom`: `a`
/// 6. `traverse_expr`: `1`
/// 7. `visit_literal_value`: `1`
/// 8. `visit_arithmetic`: `(+ a 1)`
///
/// When implementing the `ASTVisitor` trait, the default `traverse_*` methods
/// should be used when possible, implementing only the `visit_*` methods.
/// `traverse_*` methods should only be overridden when some action must be
/// taken before the sub-expressions are visited.
pub trait ASTVisitor {
    fn traverse_expr<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        match &expr.expr {
            AtomValue(value) => self.visit_atom_value(builder, expr, value),
            Atom(name) => self.visit_atom(builder, expr, name),
            List(exprs) => self.traverse_list(builder, expr, exprs),
            LiteralValue(value) => self.visit_literal_value(builder, expr, value),
            Field(field) => self.visit_field(builder, expr, field),
            TraitReference(name, trait_def) => {
                self.visit_trait_reference(builder, expr, name, trait_def)
            }
        }
    }

    // AST level traverse/visit methods

    fn traverse_list<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        list: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        if let Some((function_name, args)) = list.split_first() {
            if let Some(function_name) = function_name.match_atom() {
                if let Some(define_function) = DefineFunctions::lookup_by_name(function_name) {
                    builder = match define_function {
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
                                _ => Err(builder),
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
                    builder = match native_function {
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
                                _ => Err(builder),
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
                            let key = match_tuple(args.get(1).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_else(|| {
                                    let mut tuple_map = HashMap::new();
                                    tuple_map.insert(None, args.get(1).unwrap_or(&DEFAULT_EXPR));
                                    tuple_map
                                });
                            self.traverse_map_get(builder, expr, name, &key)
                        }
                        SetEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            let key = match_tuple(args.get(1).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_else(|| {
                                    let mut tuple_map = HashMap::new();
                                    tuple_map.insert(None, args.get(1).unwrap_or(&DEFAULT_EXPR));
                                    tuple_map
                                });
                            let value = match_tuple(args.get(2).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_else(|| {
                                    let mut tuple_map = HashMap::new();
                                    tuple_map.insert(None, args.get(2).unwrap_or(&DEFAULT_EXPR));
                                    tuple_map
                                });
                            self.traverse_map_set(builder, expr, name, &key, &value)
                        }
                        InsertEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            let key = match_tuple(args.get(1).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_else(|| {
                                    let mut tuple_map = HashMap::new();
                                    tuple_map.insert(None, args.get(1).unwrap_or(&DEFAULT_EXPR));
                                    tuple_map
                                });
                            let value = match_tuple(args.get(2).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_else(|| {
                                    let mut tuple_map = HashMap::new();
                                    tuple_map.insert(None, args.get(2).unwrap_or(&DEFAULT_EXPR));
                                    tuple_map
                                });
                            self.traverse_map_insert(builder, expr, name, &key, &value)
                        }
                        DeleteEntry => {
                            let name = args
                                .get(0)
                                .unwrap_or(&DEFAULT_EXPR)
                                .match_atom()
                                .unwrap_or(&DEFAULT_NAME);
                            let key = match_tuple(args.get(1).unwrap_or(&DEFAULT_EXPR))
                                .unwrap_or_else(|| {
                                    let mut tuple_map = HashMap::new();
                                    tuple_map.insert(None, args.get(1).unwrap_or(&DEFAULT_EXPR));
                                    tuple_map
                                });
                            self.traverse_map_delete(builder, expr, name, &key)
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
                        BitwiseAnd | BitwiseOr | BitwiseNot | BitwiseXor2 => {
                            self.traverse_bitwise(builder, expr, native_function, args)
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
                    builder =
                        self.traverse_call_user_defined(builder, expr, function_name, args)?;
                }
            }
        }
        self.visit_list(builder, expr, list)
    }

    fn visit_list<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _list: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_atom_value<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &Value,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_atom<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _atom: &'c ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_literal_value<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &Value,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_field<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _field: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_trait_reference<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _trait_def: &TraitDefinition,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    // Higher level traverse/visit methods

    fn traverse_define_constant<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_define_constant(builder, expr, name, value)
    }

    fn visit_define_constant<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_private<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        parameters: Option<Vec<TypedVar<'c>>>,
        body: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, body)?;
        self.visit_define_private(builder, expr, name, parameters, body)
    }

    fn visit_define_private<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _parameters: Option<Vec<TypedVar<'c>>>,
        _body: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_read_only<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        parameters: Option<Vec<TypedVar<'c>>>,
        body: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, body)?;
        self.visit_define_read_only(builder, expr, name, parameters, body)
    }

    fn visit_define_read_only<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _parameters: Option<Vec<TypedVar<'c>>>,
        _body: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_public<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        parameters: Option<Vec<TypedVar<'c>>>,
        body: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, body)?;
        self.visit_define_public(builder, expr, name, parameters, body)
    }

    fn visit_define_public<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _parameters: Option<Vec<TypedVar<'c>>>,
        _body: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_nft<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        nft_type: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_define_nft(builder, expr, name, nft_type)
    }

    fn visit_define_nft<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _nft_type: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_ft<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        supply: Option<&'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        if let Some(supply_expr) = supply {
            builder = self.traverse_expr(builder, supply_expr)?;
        }

        self.visit_define_ft(builder, expr, name, supply)
    }

    fn visit_define_ft<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _supply: Option<&'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_map<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        key_type: &'c SymbolicExpression,
        value_type: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_define_map(builder, expr, name, key_type, value_type)
    }

    fn visit_define_map<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _key_type: &'c SymbolicExpression,
        _value_type: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_data_var<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        data_type: &'c SymbolicExpression,
        initial: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, initial)?;
        self.visit_define_data_var(builder, expr, name, data_type, initial)
    }

    fn visit_define_data_var<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _data_type: &'c SymbolicExpression,
        _initial: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_trait<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        functions: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_define_trait(builder, expr, name, functions)
    }

    fn visit_define_trait<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _functions: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_use_trait<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_use_trait(builder, expr, name, trait_identifier)
    }

    fn visit_use_trait<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_impl_trait<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_impl_trait(builder, expr, trait_identifier)
    }

    fn visit_impl_trait<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_arithmetic<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: NativeFunctions,
        operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_arithmetic(builder, expr, func, operands)
    }

    fn visit_arithmetic<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: NativeFunctions,
        _operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_binary_bitwise<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: NativeFunctions,
        lhs: &'c SymbolicExpression,
        rhs: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in &[lhs, rhs] {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_binary_bitwise(builder, expr, func, lhs, rhs)
    }

    fn visit_binary_bitwise<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: NativeFunctions,
        _lhs: &'c SymbolicExpression,
        _rhs: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_comparison<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: NativeFunctions,
        operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_comparison(builder, expr, func, operands)
    }

    fn visit_comparison<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: NativeFunctions,
        _operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_lazy_logical<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        function: NativeFunctions,
        operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_lazy_logical(builder, expr, function, operands)
    }

    fn visit_lazy_logical<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _function: NativeFunctions,
        _operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_logical<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        function: NativeFunctions,
        operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_logical(builder, expr, function, operands)
    }

    fn visit_logical<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _function: NativeFunctions,
        _operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_int_cast<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_int_cast(builder, expr, input)
    }

    fn visit_int_cast<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_if<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        cond: &'c SymbolicExpression,
        then_expr: &'c SymbolicExpression,
        else_expr: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for &expr in &[cond, then_expr, else_expr] {
            builder = self.traverse_expr(builder, expr)?;
        }
        self.visit_if(builder, expr, cond, then_expr, else_expr)
    }

    fn visit_if<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _cond: &'c SymbolicExpression,
        _then_expr: &'c SymbolicExpression,
        _else_expr: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_var_get<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_var_get(builder, expr, name)
    }

    fn visit_var_get<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_var_set<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_var_set(builder, expr, name, value)
    }

    fn visit_var_set<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_get<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in key.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        self.visit_map_get(builder, expr, name, key)
    }

    fn visit_map_get<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_set<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
        value: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for key_val in key.values() {
            builder = self.traverse_expr(builder, key_val)?;
        }
        for val_val in value.values() {
            builder = self.traverse_expr(builder, val_val)?;
        }
        self.visit_map_set(builder, expr, name, key, value)
    }

    fn visit_map_set<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
        _value: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_insert<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
        value: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for key_val in key.values() {
            builder = self.traverse_expr(builder, key_val)?;
        }
        for val_val in value.values() {
            builder = self.traverse_expr(builder, val_val)?;
        }
        self.visit_map_insert(builder, expr, name, key, value)
    }

    fn visit_map_insert<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
        _value: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_delete<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in key.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        self.visit_map_delete(builder, expr, name, key)
    }

    fn visit_map_delete<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _key: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_tuple<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        values: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in values.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        self.visit_tuple(builder, expr, values)
    }

    fn visit_tuple<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _values: &HashMap<Option<&'c ClarityName>, &'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_get<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        key: &'c ClarityName,
        tuple: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, tuple)?;
        self.visit_get(builder, expr, key, tuple)
    }

    fn visit_get<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _key: &'c ClarityName,
        _tuple: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_merge<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        tuple1: &'c SymbolicExpression,
        tuple2: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, tuple1)?;
        builder = self.traverse_expr(builder, tuple2)?;
        self.visit_merge(builder, expr, tuple1, tuple2)
    }

    fn visit_merge<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _tuple1: &'c SymbolicExpression,
        _tuple2: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_begin<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        statements: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for stmt in statements {
            builder = self.traverse_expr(builder, stmt)?;
        }
        self.visit_begin(builder, expr, statements)
    }

    fn visit_begin<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _statements: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_hash<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: NativeFunctions,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_hash(builder, expr, func, value)
    }

    fn visit_hash<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: NativeFunctions,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_secp256k1_recover<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        hash: &'c SymbolicExpression,
        signature: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, hash)?;
        builder = self.traverse_expr(builder, signature)?;
        self.visit_secp256k1_recover(builder, expr, hash, signature)
    }

    fn visit_secp256k1_recover<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _hash: &'c SymbolicExpression,
        _signature: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_secp256k1_verify<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        hash: &'c SymbolicExpression,
        signature: &'c SymbolicExpression,
        public_key: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, hash)?;
        builder = self.traverse_expr(builder, signature)?;
        self.visit_secp256k1_verify(builder, expr, hash, signature, public_key)
    }

    fn visit_secp256k1_verify<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _hash: &'c SymbolicExpression,
        _signature: &'c SymbolicExpression,
        _public_key: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_print<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_print(builder, expr, value)
    }

    fn visit_print<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_static_contract_call<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        contract_identifier: &'c QualifiedContractIdentifier,
        function_name: &'c ClarityName,
        args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_static_contract_call(builder, expr, contract_identifier, function_name, args)
    }

    fn visit_static_contract_call<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _contract_identifier: &'c QualifiedContractIdentifier,
        _function_name: &'c ClarityName,
        _args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_dynamic_contract_call<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        trait_ref: &'c SymbolicExpression,
        function_name: &'c ClarityName,
        args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, trait_ref)?;
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_dynamic_contract_call(builder, expr, trait_ref, function_name, args)
    }

    fn visit_dynamic_contract_call<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _trait_ref: &'c SymbolicExpression,
        _function_name: &'c ClarityName,
        _args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_as_contract<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        inner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, inner)?;
        self.visit_as_contract(builder, expr, inner)
    }

    fn visit_as_contract<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _inner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_contract_of<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, name)?;
        self.visit_contract_of(builder, expr, name)
    }

    fn visit_contract_of<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_principal_of<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        public_key: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, public_key)?;
        self.visit_principal_of(builder, expr, public_key)
    }

    fn visit_principal_of<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _public_key: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_at_block<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        block: &'c SymbolicExpression,
        inner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, block)?;
        builder = self.traverse_expr(builder, inner)?;
        self.visit_at_block(builder, expr, block, inner)
    }

    fn visit_at_block<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _block: &'c SymbolicExpression,
        _inner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_get_block_info<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        prop_name: &'c ClarityName,
        block: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, block)?;
        self.visit_get_block_info(builder, expr, prop_name, block)
    }

    fn visit_get_block_info<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _prop_name: &'c ClarityName,
        _block: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_err<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_err(builder, expr, value)
    }

    fn visit_err<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ok<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_ok(builder, expr, value)
    }

    fn visit_ok<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_some<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_some(builder, expr, value)
    }

    fn visit_some<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_default_to<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        default: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, default)?;
        builder = self.traverse_expr(builder, value)?;
        self.visit_default_to(builder, expr, default, value)
    }

    fn visit_default_to<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _default: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
        throws: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, throws)?;
        self.visit_unwrap(builder, expr, input, throws)
    }

    fn visit_unwrap<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
        _throws: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap_err<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
        throws: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, throws)?;
        self.visit_unwrap_err(builder, expr, input, throws)
    }

    fn visit_unwrap_err<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
        _throws: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_ok<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_ok(builder, expr, value)
    }

    fn visit_is_ok<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_none<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_none(builder, expr, value)
    }

    fn visit_is_none<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_err<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_err(builder, expr, value)
    }

    fn visit_is_err<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_some<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_some(builder, expr, value)
    }

    fn visit_is_some<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_filter<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: &'c ClarityName,
        sequence: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        self.visit_filter(builder, expr, func, sequence)
    }

    fn visit_filter<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: &'c ClarityName,
        _sequence: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap_panic<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_unwrap_panic(builder, expr, input)
    }

    fn visit_unwrap_panic<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap_err_panic<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_unwrap_err_panic(builder, expr, input)
    }

    fn visit_unwrap_err_panic<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_match_option<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
        some_name: &'c ClarityName,
        some_branch: &'c SymbolicExpression,
        none_branch: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, some_branch)?;
        builder = self.traverse_expr(builder, none_branch)?;
        self.visit_match_option(builder, expr, input, some_name, some_branch, none_branch)
    }

    fn visit_match_option<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
        _some_name: &'c ClarityName,
        _some_branch: &'c SymbolicExpression,
        _none_branch: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    #[allow(clippy::too_many_arguments)]
    fn traverse_match_response<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
        ok_name: &'c ClarityName,
        ok_branch: &'c SymbolicExpression,
        err_name: &'c ClarityName,
        err_branch: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, ok_branch)?;
        builder = self.traverse_expr(builder, err_branch)?;
        self.visit_match_response(
            builder, expr, input, ok_name, ok_branch, err_name, err_branch,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn visit_match_response<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
        _ok_name: &'c ClarityName,
        _ok_branch: &'c SymbolicExpression,
        _err_name: &'c ClarityName,
        _err_branch: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_try<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_try(builder, expr, input)
    }

    fn visit_try<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_asserts<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        cond: &'c SymbolicExpression,
        thrown: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, cond)?;
        builder = self.traverse_expr(builder, thrown)?;
        self.visit_asserts(builder, expr, cond, thrown)
    }

    fn visit_asserts<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _cond: &'c SymbolicExpression,
        _thrown: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_burn<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        amount: &'c SymbolicExpression,
        sender: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        self.visit_stx_burn(builder, expr, amount, sender)
    }

    fn visit_stx_burn<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _amount: &'c SymbolicExpression,
        _sender: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_transfer<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        amount: &'c SymbolicExpression,
        sender: &'c SymbolicExpression,
        recipient: &'c SymbolicExpression,
        memo: Option<&'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        builder = self.traverse_expr(builder, recipient)?;
        if let Some(memo) = memo {
            builder = self.traverse_expr(builder, memo)?;
        }
        self.visit_stx_transfer(builder, expr, amount, sender, recipient, memo)
    }

    fn visit_stx_transfer<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _amount: &'c SymbolicExpression,
        _sender: &'c SymbolicExpression,
        _recipient: &'c SymbolicExpression,
        _memo: Option<&'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_get_balance<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        owner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, owner)?;
        self.visit_stx_get_balance(builder, expr, owner)
    }

    fn visit_stx_get_balance<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _owner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_burn<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        amount: &'c SymbolicExpression,
        sender: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        self.visit_ft_burn(builder, expr, token, amount, sender)
    }

    fn visit_ft_burn<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _amount: &'c SymbolicExpression,
        _sender: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_transfer<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        amount: &'c SymbolicExpression,
        sender: &'c SymbolicExpression,
        recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_ft_transfer(builder, expr, token, amount, sender, recipient)
    }

    fn visit_ft_transfer<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _amount: &'c SymbolicExpression,
        _sender: &'c SymbolicExpression,
        _recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_get_balance<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        owner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, owner)?;
        self.visit_ft_get_balance(builder, expr, token, owner)
    }

    fn visit_ft_get_balance<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _owner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_get_supply<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_ft_get_supply(builder, expr, token)
    }

    fn visit_ft_get_supply<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_mint<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        amount: &'c SymbolicExpression,
        recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_ft_mint(builder, expr, token, amount, recipient)
    }

    fn visit_ft_mint<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _amount: &'c SymbolicExpression,
        _recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_burn<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        identifier: &'c SymbolicExpression,
        sender: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        builder = self.traverse_expr(builder, sender)?;
        self.visit_nft_burn(builder, expr, token, identifier, sender)
    }

    fn visit_nft_burn<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _identifier: &'c SymbolicExpression,
        _sender: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_transfer<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        identifier: &'c SymbolicExpression,
        sender: &'c SymbolicExpression,
        recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        builder = self.traverse_expr(builder, sender)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_nft_transfer(builder, expr, token, identifier, sender, recipient)
    }

    fn visit_nft_transfer<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _identifier: &'c SymbolicExpression,
        _sender: &'c SymbolicExpression,
        _recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_mint<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        identifier: &'c SymbolicExpression,
        recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_nft_mint(builder, expr, token, identifier, recipient)
    }

    fn visit_nft_mint<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _identifier: &'c SymbolicExpression,
        _recipient: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_get_owner<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        token: &'c ClarityName,
        identifier: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        self.visit_nft_get_owner(builder, expr, token, identifier)
    }

    fn visit_nft_get_owner<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _token: &'c ClarityName,
        _identifier: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_let<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        bindings: &HashMap<&'c ClarityName, &'c SymbolicExpression>,
        body: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in bindings.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        for expr in body {
            builder = self.traverse_expr(builder, expr)?;
        }
        self.visit_let(builder, expr, bindings, body)
    }

    fn visit_let<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _bindings: &HashMap<&'c ClarityName, &'c SymbolicExpression>,
        _body: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: &'c ClarityName,
        sequences: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for sequence in sequences {
            builder = self.traverse_expr(builder, sequence)?;
        }
        self.visit_map(builder, expr, func, sequences)
    }

    fn visit_map<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: &'c ClarityName,
        _sequences: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_fold<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: &'c ClarityName,
        sequence: &'c SymbolicExpression,
        initial: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, initial)?;
        self.visit_fold(builder, expr, func, sequence, initial)
    }

    fn visit_fold<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: &'c ClarityName,
        _sequence: &'c SymbolicExpression,
        _initial: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_append<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        list: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, list)?;
        builder = self.traverse_expr(builder, value)?;
        self.visit_append(builder, expr, list, value)
    }

    fn visit_append<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _list: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_concat<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        lhs: &'c SymbolicExpression,
        rhs: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, lhs)?;
        builder = self.traverse_expr(builder, rhs)?;
        self.visit_concat(builder, expr, lhs, rhs)
    }

    fn visit_concat<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _lhs: &'c SymbolicExpression,
        _rhs: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_as_max_len<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        sequence: &'c SymbolicExpression,
        length: u128,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        self.visit_as_max_len(builder, expr, sequence, length)
    }

    fn visit_as_max_len<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _sequence: &'c SymbolicExpression,
        _length: u128,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_len<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        sequence: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        self.visit_len(builder, expr, sequence)
    }

    fn visit_len<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _sequence: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_element_at<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        sequence: &'c SymbolicExpression,
        index: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, index)?;
        self.visit_element_at(builder, expr, sequence, index)
    }

    fn visit_element_at<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _sequence: &'c SymbolicExpression,
        _index: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_index_of<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        sequence: &'c SymbolicExpression,
        item: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, item)?;
        self.visit_element_at(builder, expr, sequence, item)
    }

    fn visit_index_of<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _sequence: &'c SymbolicExpression,
        _item: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_list_cons<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_list_cons(builder, expr, args)
    }

    fn visit_list_cons<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_call_user_defined<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        name: &'c ClarityName,
        args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_call_user_defined(builder, expr, name, args)
    }

    fn visit_call_user_defined<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _name: &'c ClarityName,
        _args: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_buff_cast<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_buff_cast(builder, expr, input)
    }

    fn visit_buff_cast<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_standard<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_standard(builder, expr, value)
    }

    fn visit_is_standard<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _value: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_principal_destruct<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        principal: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, principal)?;
        self.visit_principal_destruct(builder, expr, principal)
    }

    fn visit_principal_destruct<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _principal: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_principal_construct<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        buff1: &'c SymbolicExpression,
        buff20: &'c SymbolicExpression,
        contract: Option<&'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, buff1)?;
        builder = self.traverse_expr(builder, buff20)?;
        if let Some(contract) = contract {
            builder = self.traverse_expr(builder, contract)?;
        }
        self.visit_principal_construct(builder, expr, buff1, buff20, contract)
    }

    fn visit_principal_construct<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _buff1: &'c SymbolicExpression,
        _buff20: &'c SymbolicExpression,
        _contract: Option<&'c SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_string_to_int<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_string_to_int(builder, expr, input)
    }

    fn visit_string_to_int<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_int_to_string<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_int_to_string(builder, expr, input)
    }

    fn visit_int_to_string<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_get_account<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        owner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, owner)?;
        self.visit_stx_get_account(builder, expr, owner)
    }

    fn visit_stx_get_account<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _owner: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_slice<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        seq: &'c SymbolicExpression,
        left: &'c SymbolicExpression,
        right: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, seq)?;
        builder = self.traverse_expr(builder, left)?;
        builder = self.traverse_expr(builder, right)?;
        self.visit_slice(builder, expr, seq, left, right)
    }

    fn visit_slice<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _seq: &'c SymbolicExpression,
        _left: &'c SymbolicExpression,
        _right: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_get_burn_block_info<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        prop_name: &'c ClarityName,
        block: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, block)?;
        self.visit_get_burn_block_info(builder, expr, prop_name, block)
    }

    fn visit_get_burn_block_info<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _prop_name: &'c ClarityName,
        _block: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_to_consensus_buff<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_to_consensus_buff(builder, expr, input)
    }

    fn visit_to_consensus_buff<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_from_consensus_buff<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        type_expr: &'c SymbolicExpression,
        input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, type_expr)?;
        builder = self.traverse_expr(builder, input)?;
        self.visit_from_consensus_buff(builder, expr, type_expr, input)
    }

    fn visit_from_consensus_buff<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _type_expr: &'c SymbolicExpression,
        _input: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_bitwise<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: NativeFunctions,
        operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_bitwise(builder, expr, func, operands)
    }

    fn visit_bitwise<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: NativeFunctions,
        _operands: &'c [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_replace_at<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        sequence: &'c SymbolicExpression,
        index: &'c SymbolicExpression,
        element: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, index)?;
        builder = self.traverse_expr(builder, element)?;
        self.visit_replace_at(builder, expr, sequence, element, index)
    }

    fn visit_replace_at<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _sequence: &'c SymbolicExpression,
        _index: &'c SymbolicExpression,
        _element: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_bit_shift<'b, 'c>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'c SymbolicExpression,
        func: NativeFunctions,
        input: &'c SymbolicExpression,
        shamt: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, shamt)?;
        self.visit_bit_shift(builder, expr, func, input, shamt)
    }

    fn visit_bit_shift<'b, 'c>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'c SymbolicExpression,
        _func: NativeFunctions,
        _input: &'c SymbolicExpression,
        _shamt: &'c SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }
}

pub fn traverse<'a, 'b>(
    visitor: &'a mut impl ASTVisitor,
    mut builder: InstrSeqBuilder<'b>,
    exprs: &[SymbolicExpression],
) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
    for expr in exprs {
        builder = visitor.traverse_expr(builder, expr)?;
    }
    Ok(builder)
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

fn match_pairs_list(list: &[SymbolicExpression]) -> Option<Vec<TypedVar>> {
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
