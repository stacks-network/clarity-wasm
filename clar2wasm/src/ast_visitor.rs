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
pub struct TypedVar<'a> {
    pub name: &'a ClarityName,
    pub type_expr: &'a SymbolicExpression,
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
pub trait ASTVisitor<'a> {
    fn traverse_expr<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
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

    fn traverse_list<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        list: &'a [SymbolicExpression],
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

    fn visit_list<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _list: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_atom_value<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &Value,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_atom<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _atom: &'a ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_literal_value<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &Value,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_field<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _field: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn visit_trait_reference<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _trait_def: &TraitDefinition,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    // Higher level traverse/visit methods

    fn traverse_define_constant<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_define_constant(builder, expr, name, value)
    }

    fn visit_define_constant<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_private<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, body)?;
        self.visit_define_private(builder, expr, name, parameters, body)
    }

    fn visit_define_private<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _parameters: Option<Vec<TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_read_only<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, body)?;
        self.visit_define_read_only(builder, expr, name, parameters, body)
    }

    fn visit_define_read_only<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _parameters: Option<Vec<TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_public<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, body)?;
        self.visit_define_public(builder, expr, name, parameters, body)
    }

    fn visit_define_public<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _parameters: Option<Vec<TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_nft<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        nft_type: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_define_nft(builder, expr, name, nft_type)
    }

    fn visit_define_nft<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _nft_type: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_ft<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        supply: Option<&'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        if let Some(supply_expr) = supply {
            builder = self.traverse_expr(builder, supply_expr)?;
        }

        self.visit_define_ft(builder, expr, name, supply)
    }

    fn visit_define_ft<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _supply: Option<&'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_map<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        key_type: &'a SymbolicExpression,
        value_type: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_define_map(builder, expr, name, key_type, value_type)
    }

    fn visit_define_map<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _key_type: &'a SymbolicExpression,
        _value_type: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_data_var<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        data_type: &'a SymbolicExpression,
        initial: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, initial)?;
        self.visit_define_data_var(builder, expr, name, data_type, initial)
    }

    fn visit_define_data_var<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _data_type: &'a SymbolicExpression,
        _initial: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_define_trait<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        functions: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_define_trait(builder, expr, name, functions)
    }

    fn visit_define_trait<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _functions: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_use_trait<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_use_trait(builder, expr, name, trait_identifier)
    }

    fn visit_use_trait<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_impl_trait<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_impl_trait(builder, expr, trait_identifier)
    }

    fn visit_impl_trait<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _trait_identifier: &TraitIdentifier,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_arithmetic<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_arithmetic(builder, expr, func, operands)
    }

    fn visit_arithmetic<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: NativeFunctions,
        _operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_binary_bitwise<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        lhs: &'a SymbolicExpression,
        rhs: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in &[lhs, rhs] {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_binary_bitwise(builder, expr, func, lhs, rhs)
    }

    fn visit_binary_bitwise<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: NativeFunctions,
        _lhs: &'a SymbolicExpression,
        _rhs: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_comparison<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_comparison(builder, expr, func, operands)
    }

    fn visit_comparison<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: NativeFunctions,
        _operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_lazy_logical<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        function: NativeFunctions,
        operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_lazy_logical(builder, expr, function, operands)
    }

    fn visit_lazy_logical<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _function: NativeFunctions,
        _operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_logical<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        function: NativeFunctions,
        operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_logical(builder, expr, function, operands)
    }

    fn visit_logical<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _function: NativeFunctions,
        _operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_int_cast<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_int_cast(builder, expr, input)
    }

    fn visit_int_cast<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_if<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        cond: &'a SymbolicExpression,
        then_expr: &'a SymbolicExpression,
        else_expr: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for &expr in &[cond, then_expr, else_expr] {
            builder = self.traverse_expr(builder, expr)?;
        }
        self.visit_if(builder, expr, cond, then_expr, else_expr)
    }

    fn visit_if<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _cond: &'a SymbolicExpression,
        _then_expr: &'a SymbolicExpression,
        _else_expr: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_var_get<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_var_get(builder, expr, name)
    }

    fn visit_var_get<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_var_set<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_var_set(builder, expr, name, value)
    }

    fn visit_var_set<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_get<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in key.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        self.visit_map_get(builder, expr, name, key)
    }

    fn visit_map_get<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_set<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
        value: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for key_val in key.values() {
            builder = self.traverse_expr(builder, key_val)?;
        }
        for val_val in value.values() {
            builder = self.traverse_expr(builder, val_val)?;
        }
        self.visit_map_set(builder, expr, name, key, value)
    }

    fn visit_map_set<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
        _value: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_insert<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
        value: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for key_val in key.values() {
            builder = self.traverse_expr(builder, key_val)?;
        }
        for val_val in value.values() {
            builder = self.traverse_expr(builder, val_val)?;
        }
        self.visit_map_insert(builder, expr, name, key, value)
    }

    fn visit_map_insert<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
        _value: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map_delete<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in key.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        self.visit_map_delete(builder, expr, name, key)
    }

    fn visit_map_delete<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _key: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_tuple<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        values: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in values.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        self.visit_tuple(builder, expr, values)
    }

    fn visit_tuple<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _values: &HashMap<Option<&'a ClarityName>, &'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_get<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        key: &'a ClarityName,
        tuple: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, tuple)?;
        self.visit_get(builder, expr, key, tuple)
    }

    fn visit_get<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _key: &'a ClarityName,
        _tuple: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_merge<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        tuple1: &'a SymbolicExpression,
        tuple2: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, tuple1)?;
        builder = self.traverse_expr(builder, tuple2)?;
        self.visit_merge(builder, expr, tuple1, tuple2)
    }

    fn visit_merge<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _tuple1: &'a SymbolicExpression,
        _tuple2: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_begin<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        statements: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for stmt in statements {
            builder = self.traverse_expr(builder, stmt)?;
        }
        self.visit_begin(builder, expr, statements)
    }

    fn visit_begin<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _statements: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_hash<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_hash(builder, expr, func, value)
    }

    fn visit_hash<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: NativeFunctions,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_secp256k1_recover<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        hash: &'a SymbolicExpression,
        signature: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, hash)?;
        builder = self.traverse_expr(builder, signature)?;
        self.visit_secp256k1_recover(builder, expr, hash, signature)
    }

    fn visit_secp256k1_recover<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _hash: &SymbolicExpression,
        _signature: &SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_secp256k1_verify<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        hash: &'a SymbolicExpression,
        signature: &'a SymbolicExpression,
        public_key: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, hash)?;
        builder = self.traverse_expr(builder, signature)?;
        self.visit_secp256k1_verify(builder, expr, hash, signature, public_key)
    }

    fn visit_secp256k1_verify<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _hash: &SymbolicExpression,
        _signature: &SymbolicExpression,
        _public_key: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_print<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_print(builder, expr, value)
    }

    fn visit_print<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_static_contract_call<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        contract_identifier: &'a QualifiedContractIdentifier,
        function_name: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_static_contract_call(builder, expr, contract_identifier, function_name, args)
    }

    fn visit_static_contract_call<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _contract_identifier: &'a QualifiedContractIdentifier,
        _function_name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_dynamic_contract_call<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        trait_ref: &'a SymbolicExpression,
        function_name: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, trait_ref)?;
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_dynamic_contract_call(builder, expr, trait_ref, function_name, args)
    }

    fn visit_dynamic_contract_call<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _trait_ref: &'a SymbolicExpression,
        _function_name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_as_contract<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        inner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, inner)?;
        self.visit_as_contract(builder, expr, inner)
    }

    fn visit_as_contract<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _inner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_contract_of<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, name)?;
        self.visit_contract_of(builder, expr, name)
    }

    fn visit_contract_of<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_principal_of<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        public_key: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, public_key)?;
        self.visit_principal_of(builder, expr, public_key)
    }

    fn visit_principal_of<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _public_key: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_at_block<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        block: &'a SymbolicExpression,
        inner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, block)?;
        builder = self.traverse_expr(builder, inner)?;
        self.visit_at_block(builder, expr, block, inner)
    }

    fn visit_at_block<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _block: &'a SymbolicExpression,
        _inner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_get_block_info<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        prop_name: &'a ClarityName,
        block: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, block)?;
        self.visit_get_block_info(builder, expr, prop_name, block)
    }

    fn visit_get_block_info<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _prop_name: &'a ClarityName,
        _block: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_err<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_err(builder, expr, value)
    }

    fn visit_err<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ok<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_ok(builder, expr, value)
    }

    fn visit_ok<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_some<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_some(builder, expr, value)
    }

    fn visit_some<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_default_to<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        default: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, default)?;
        builder = self.traverse_expr(builder, value)?;
        self.visit_default_to(builder, expr, default, value)
    }

    fn visit_default_to<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _default: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
        throws: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, throws)?;
        self.visit_unwrap(builder, expr, input, throws)
    }

    fn visit_unwrap<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
        _throws: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap_err<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
        throws: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, throws)?;
        self.visit_unwrap_err(builder, expr, input, throws)
    }

    fn visit_unwrap_err<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
        _throws: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_ok<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_ok(builder, expr, value)
    }

    fn visit_is_ok<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_none<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_none(builder, expr, value)
    }

    fn visit_is_none<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_err<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_err(builder, expr, value)
    }

    fn visit_is_err<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_some<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_some(builder, expr, value)
    }

    fn visit_is_some<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_filter<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: &'a ClarityName,
        sequence: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        self.visit_filter(builder, expr, func, sequence)
    }

    fn visit_filter<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: &'a ClarityName,
        _sequence: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap_panic<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_unwrap_panic(builder, expr, input)
    }

    fn visit_unwrap_panic<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_unwrap_err_panic<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_unwrap_err_panic(builder, expr, input)
    }

    fn visit_unwrap_err_panic<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_match_option<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
        some_name: &'a ClarityName,
        some_branch: &'a SymbolicExpression,
        none_branch: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, some_branch)?;
        builder = self.traverse_expr(builder, none_branch)?;
        self.visit_match_option(builder, expr, input, some_name, some_branch, none_branch)
    }

    fn visit_match_option<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
        _some_name: &'a ClarityName,
        _some_branch: &'a SymbolicExpression,
        _none_branch: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    #[allow(clippy::too_many_arguments)]
    fn traverse_match_response<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
        ok_name: &'a ClarityName,
        ok_branch: &'a SymbolicExpression,
        err_name: &'a ClarityName,
        err_branch: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, ok_branch)?;
        builder = self.traverse_expr(builder, err_branch)?;
        self.visit_match_response(
            builder, expr, input, ok_name, ok_branch, err_name, err_branch,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn visit_match_response<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
        _ok_name: &'a ClarityName,
        _ok_branch: &'a SymbolicExpression,
        _err_name: &'a ClarityName,
        _err_branch: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_try<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_try(builder, expr, input)
    }

    fn visit_try<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_asserts<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        cond: &'a SymbolicExpression,
        thrown: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, cond)?;
        builder = self.traverse_expr(builder, thrown)?;
        self.visit_asserts(builder, expr, cond, thrown)
    }

    fn visit_asserts<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _cond: &'a SymbolicExpression,
        _thrown: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_burn<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        amount: &'a SymbolicExpression,
        sender: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        self.visit_stx_burn(builder, expr, amount, sender)
    }

    fn visit_stx_burn<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _amount: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_transfer<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        amount: &'a SymbolicExpression,
        sender: &'a SymbolicExpression,
        recipient: &'a SymbolicExpression,
        memo: Option<&'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        builder = self.traverse_expr(builder, recipient)?;
        if let Some(memo) = memo {
            builder = self.traverse_expr(builder, memo)?;
        }
        self.visit_stx_transfer(builder, expr, amount, sender, recipient, memo)
    }

    fn visit_stx_transfer<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _amount: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
        _memo: Option<&'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_get_balance<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        owner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, owner)?;
        self.visit_stx_get_balance(builder, expr, owner)
    }

    fn visit_stx_get_balance<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _owner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_burn<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        amount: &'a SymbolicExpression,
        sender: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        self.visit_ft_burn(builder, expr, token, amount, sender)
    }

    fn visit_ft_burn<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _amount: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_transfer<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        amount: &'a SymbolicExpression,
        sender: &'a SymbolicExpression,
        recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, sender)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_ft_transfer(builder, expr, token, amount, sender, recipient)
    }

    fn visit_ft_transfer<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _amount: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_get_balance<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        owner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, owner)?;
        self.visit_ft_get_balance(builder, expr, token, owner)
    }

    fn visit_ft_get_balance<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _owner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_get_supply<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        self.visit_ft_get_supply(builder, expr, token)
    }

    fn visit_ft_get_supply<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_ft_mint<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        amount: &'a SymbolicExpression,
        recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, amount)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_ft_mint(builder, expr, token, amount, recipient)
    }

    fn visit_ft_mint<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _amount: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_burn<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        identifier: &'a SymbolicExpression,
        sender: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        builder = self.traverse_expr(builder, sender)?;
        self.visit_nft_burn(builder, expr, token, identifier, sender)
    }

    fn visit_nft_burn<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_transfer<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        identifier: &'a SymbolicExpression,
        sender: &'a SymbolicExpression,
        recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        builder = self.traverse_expr(builder, sender)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_nft_transfer(builder, expr, token, identifier, sender, recipient)
    }

    fn visit_nft_transfer<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
        _sender: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_mint<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        identifier: &'a SymbolicExpression,
        recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        builder = self.traverse_expr(builder, recipient)?;
        self.visit_nft_mint(builder, expr, token, identifier, recipient)
    }

    fn visit_nft_mint<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
        _recipient: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_nft_get_owner<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        token: &'a ClarityName,
        identifier: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, identifier)?;
        self.visit_nft_get_owner(builder, expr, token, identifier)
    }

    fn visit_nft_get_owner<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _token: &'a ClarityName,
        _identifier: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_let<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        bindings: &HashMap<&'a ClarityName, &'a SymbolicExpression>,
        body: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for val in bindings.values() {
            builder = self.traverse_expr(builder, val)?;
        }
        for expr in body {
            builder = self.traverse_expr(builder, expr)?;
        }
        self.visit_let(builder, expr, bindings, body)
    }

    fn visit_let<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _bindings: &HashMap<&'a ClarityName, &'a SymbolicExpression>,
        _body: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_map<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: &'a ClarityName,
        sequences: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for sequence in sequences {
            builder = self.traverse_expr(builder, sequence)?;
        }
        self.visit_map(builder, expr, func, sequences)
    }

    fn visit_map<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: &'a ClarityName,
        _sequences: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_fold<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: &'a ClarityName,
        sequence: &'a SymbolicExpression,
        initial: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, initial)?;
        self.visit_fold(builder, expr, func, sequence, initial)
    }

    fn visit_fold<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: &'a ClarityName,
        _sequence: &'a SymbolicExpression,
        _initial: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_append<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        list: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, list)?;
        builder = self.traverse_expr(builder, value)?;
        self.visit_append(builder, expr, list, value)
    }

    fn visit_append<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _list: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_concat<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        lhs: &'a SymbolicExpression,
        rhs: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, lhs)?;
        builder = self.traverse_expr(builder, rhs)?;
        self.visit_concat(builder, expr, lhs, rhs)
    }

    fn visit_concat<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _lhs: &'a SymbolicExpression,
        _rhs: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_as_max_len<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        sequence: &'a SymbolicExpression,
        length: u128,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        self.visit_as_max_len(builder, expr, sequence, length)
    }

    fn visit_as_max_len<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _sequence: &'a SymbolicExpression,
        _length: u128,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_len<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        sequence: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        self.visit_len(builder, expr, sequence)
    }

    fn visit_len<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _sequence: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_element_at<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        sequence: &'a SymbolicExpression,
        index: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, index)?;
        self.visit_element_at(builder, expr, sequence, index)
    }

    fn visit_element_at<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _sequence: &'a SymbolicExpression,
        _index: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_index_of<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        sequence: &'a SymbolicExpression,
        item: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, item)?;
        self.visit_element_at(builder, expr, sequence, item)
    }

    fn visit_index_of<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _sequence: &'a SymbolicExpression,
        _item: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_list_cons<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_list_cons(builder, expr, args)
    }

    fn visit_list_cons<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_call_user_defined<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for arg in args.iter() {
            builder = self.traverse_expr(builder, arg)?;
        }
        self.visit_call_user_defined(builder, expr, name, args)
    }

    fn visit_call_user_defined<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_buff_cast<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_buff_cast(builder, expr, input)
    }

    fn visit_buff_cast<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_is_standard<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, value)?;
        self.visit_is_standard(builder, expr, value)
    }

    fn visit_is_standard<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _value: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_principal_destruct<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        principal: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, principal)?;
        self.visit_principal_destruct(builder, expr, principal)
    }

    fn visit_principal_destruct<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _principal: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_principal_construct<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        buff1: &'a SymbolicExpression,
        buff20: &'a SymbolicExpression,
        contract: Option<&'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, buff1)?;
        builder = self.traverse_expr(builder, buff20)?;
        if let Some(contract) = contract {
            builder = self.traverse_expr(builder, contract)?;
        }
        self.visit_principal_construct(builder, expr, buff1, buff20, contract)
    }

    fn visit_principal_construct<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _buff1: &'a SymbolicExpression,
        _buff20: &'a SymbolicExpression,
        _contract: Option<&'a SymbolicExpression>,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_string_to_int<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_string_to_int(builder, expr, input)
    }

    fn visit_string_to_int<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_int_to_string<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_int_to_string(builder, expr, input)
    }

    fn visit_int_to_string<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_stx_get_account<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        owner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, owner)?;
        self.visit_stx_get_account(builder, expr, owner)
    }

    fn visit_stx_get_account<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _owner: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_slice<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        seq: &'a SymbolicExpression,
        left: &'a SymbolicExpression,
        right: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, seq)?;
        builder = self.traverse_expr(builder, left)?;
        builder = self.traverse_expr(builder, right)?;
        self.visit_slice(builder, expr, seq, left, right)
    }

    fn visit_slice<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _seq: &'a SymbolicExpression,
        _left: &'a SymbolicExpression,
        _right: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_get_burn_block_info<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        prop_name: &'a ClarityName,
        block: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, block)?;
        self.visit_get_burn_block_info(builder, expr, prop_name, block)
    }

    fn visit_get_burn_block_info<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _prop_name: &'a ClarityName,
        _block: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_to_consensus_buff<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        self.visit_to_consensus_buff(builder, expr, input)
    }

    fn visit_to_consensus_buff<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_from_consensus_buff<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        type_expr: &'a SymbolicExpression,
        input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, type_expr)?;
        builder = self.traverse_expr(builder, input)?;
        self.visit_from_consensus_buff(builder, expr, type_expr, input)
    }

    fn visit_from_consensus_buff<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _type_expr: &'a SymbolicExpression,
        _input: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_bitwise<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        for operand in operands {
            builder = self.traverse_expr(builder, operand)?;
        }
        self.visit_bitwise(builder, expr, func, operands)
    }

    fn visit_bitwise<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: NativeFunctions,
        _operands: &'a [SymbolicExpression],
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_replace_at<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        sequence: &'a SymbolicExpression,
        index: &'a SymbolicExpression,
        element: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, sequence)?;
        builder = self.traverse_expr(builder, index)?;
        builder = self.traverse_expr(builder, element)?;
        self.visit_replace_at(builder, expr, sequence, element, index)
    }

    fn visit_replace_at<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _sequence: &'a SymbolicExpression,
        _index: &'a SymbolicExpression,
        _element: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }

    fn traverse_bit_shift<'b>(
        &mut self,
        mut builder: InstrSeqBuilder<'b>,
        expr: &'a SymbolicExpression,
        func: NativeFunctions,
        input: &'a SymbolicExpression,
        shamt: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        builder = self.traverse_expr(builder, input)?;
        builder = self.traverse_expr(builder, shamt)?;
        self.visit_bit_shift(builder, expr, func, input, shamt)
    }

    fn visit_bit_shift<'b>(
        &mut self,
        builder: InstrSeqBuilder<'b>,
        _expr: &'a SymbolicExpression,
        _func: NativeFunctions,
        _input: &'a SymbolicExpression,
        _shamt: &'a SymbolicExpression,
    ) -> Result<InstrSeqBuilder<'b>, InstrSeqBuilder<'b>> {
        Ok(builder)
    }
}

pub fn traverse<'a, 'b>(
    visitor: &mut impl ASTVisitor<'a>,
    mut builder: InstrSeqBuilder<'b>,
    exprs: &'a [SymbolicExpression],
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
