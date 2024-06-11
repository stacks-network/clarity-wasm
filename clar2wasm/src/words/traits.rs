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
        // Making sure if name is not reserved
        if generator.is_reserved_name(name) {
            return Err(GeneratorError::InternalError(format!(
                "Name already used {:?}",
                name
            )));
        }

        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = generator.add_string_literal(name)?;

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.define_trait")
                .ok_or_else(|| {
                    GeneratorError::InternalError("stdlib.define_trait not found".to_owned())
                })?,
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
            generator.add_string_literal(&trait_identifier.to_string())?;

        // Push the name onto the data stack
        builder
            .i32_const(trait_offset as i32)
            .i32_const(trait_length as i32);

        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.impl_trait")
                .ok_or_else(|| {
                    GeneratorError::InternalError("stdlib.impl_trait not found".to_owned())
                })?,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::types::StacksEpochId;
    use clarity::vm::types::{StandardPrincipalData, TraitIdentifier};
    use clarity::vm::Value;

    use crate::tools::{
        crosscheck, crosscheck_expect_failure, crosscheck_with_epoch, evaluate, TestEnvironment,
    };

    #[test]
    fn define_trait_eval() {
        // Just validate that it doesn't crash
        crosscheck("(define-trait my-trait ())", Ok(None))
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
            .unwrap();

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

    #[test]
    fn trait_list() {
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
        .expect("Failed to init contract my-trait.");
        let val = env.init_contract_with_snippet(
            "use-trait",
            r#"
(use-trait the-trait .my-trait.my-trait)
(define-private (foo (adder <the-trait>))
    (print (list adder adder))
)
(foo .my-trait)
            "#,
        );

        assert_eq!(val, evaluate("(list .my-trait .my-trait)"));
    }

    #[test]
    fn validate_define_trait() {
        // Reserved keyword
        crosscheck_expect_failure("(define-trait map ((func (int) (response int int))))");

        // Custom trait token name
        crosscheck(
            "(define-trait a ((func (int) (response int int))))",
            Ok(None),
        );

        // Custom trait name duplicate
        let snippet = r#"
          (define-trait a ((func (int) (response int int))))
          (define-trait a ((func (int) (response int int))))
        "#;
        crosscheck_expect_failure(snippet);
    }

    #[test]
    fn validate_define_trait_epoch() {
        // Epoch20
        crosscheck_with_epoch(
            "(define-trait index-of ((func (int) (response int int))))",
            Err(()),
            StacksEpochId::Epoch20,
        );
        crosscheck_with_epoch(
            "(define-trait index-of? ((func (int) (response int int))))",
            Ok(None),
            StacksEpochId::Epoch20,
        );

        crosscheck_expect_failure("(define-trait index-of? ((func (int) (response int int))))");
    }
}
