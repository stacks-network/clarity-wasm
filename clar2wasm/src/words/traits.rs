use clarity::vm::{ClarityName, SymbolicExpression, SymbolicExpressionType};

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};

#[derive(Debug)]
pub struct DefineTrait;

impl ComplexWord for DefineTrait {
    fn name(&self) -> ClarityName {
        "define-trait".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;

        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = generator.add_string_literal(name);

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.define_trait")
                .expect("function not found"),
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct UseTrait;

impl ComplexWord for UseTrait {
    fn name(&self) -> ClarityName {
        "use-trait".into()
    }

    fn traverse(
        &self,
        _generator: &mut WasmGenerator,
        _builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        _args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // This is just for the type-checker, so it's a no-op at runtime.
        Ok(())
    }
}

#[derive(Debug)]
pub struct ImplTrait;

impl ComplexWord for ImplTrait {
    fn name(&self) -> ClarityName {
        "impl-trait".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let trait_identifier = match &args.get_expr(0)?.expr {
            SymbolicExpressionType::Field(trait_identifier) => trait_identifier,
            _ => {
                return Err(GeneratorError::TypeError(
                    "Expected trait identifier".into(),
                ))
            }
        };

        // Store the trait identifier as a string literal in the memory
        let (trait_offset, trait_length) =
            generator.add_string_literal(&trait_identifier.to_string());

        // Push the name onto the data stack
        builder
            .i32_const(trait_offset as i32)
            .i32_const(trait_length as i32);

        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.impl_trait")
                .expect("function not found"),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::{StandardPrincipalData, TraitIdentifier};
    use clarity::vm::Value;

    use crate::tools::{evaluate, TestEnvironment};

    #[test]
    fn define_trait_eval() {
        // Just validate that it doesn't crash
        assert_eq!(evaluate("(define-trait my-trait ())"), None);
    }

    #[test]
    fn define_trait_check_context() {
        let mut env = TestEnvironment::default();
        let val = env
            .init_contract_with_snippet(
                "token-trait",
                r#"
(define-trait token-trait
    ((transfer? (principal principal uint) (response uint uint))
        (get-balance (principal) (response uint uint))))
             "#,
            )
            .expect("Failed to init contract.");

        assert!(val.is_none());
        let contract_context = env.get_contract_context("token-trait").unwrap();
        let token_trait = contract_context
            .lookup_trait_definition("token-trait")
            .unwrap();
        assert_eq!(token_trait.len(), 2);
    }

    #[test]
    fn use_trait_eval() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "my-trait",
            r#"
(define-trait my-trait
    ((add (int int) (response int int))))
             "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet("use-token", "(use-trait the-trait .my-trait.my-trait)")
            .expect("Failed to init contract.");

        assert!(val.is_none());
    }

    #[test]
    fn use_trait_call() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "my-trait",
            r#"
(define-trait my-trait
  ((add (int int) (response int int))))
(define-public (add (a int) (b int))
  (ok (+ a b))
)
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "use-trait",
                r#"
(use-trait the-trait .my-trait.my-trait)
(define-private (foo (adder <the-trait>) (a int) (b int))
    (contract-call? adder add a b)
)
(foo .my-trait 1 2)
            "#,
            )
            .expect("Failed to init contract.");

        assert_eq!(val.unwrap(), Value::okay(Value::Int(3)).unwrap());
    }

    #[test]
    fn impl_trait_eval() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "my-trait",
            r#"
(define-trait my-trait
  ((add (int int) (response int int))))
            "#,
        )
        .expect("Failed to init contract.");
        let val = env
            .init_contract_with_snippet(
                "impl-trait",
                r#"
(impl-trait .my-trait.my-trait)
(define-public (add (a int) (b int))
  (ok (+ a b))
)
            "#,
            )
            .expect("Failed to init contract.");

        assert!(val.is_none());

        let contract_context = env.get_contract_context("impl-trait").unwrap();
        assert!(contract_context
            .implemented_traits
            .get(&TraitIdentifier::new(
                StandardPrincipalData::transient(),
                "my-trait".into(),
                "my-trait".into(),
            ))
            .is_some());
    }
}
