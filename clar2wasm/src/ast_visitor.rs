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
    // AST level traverse/visit methods
}
