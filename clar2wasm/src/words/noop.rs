use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{types::Value, ClarityName, SymbolicExpression, SymbolicExpressionType};

use super::Word;

// Functions below are considered no-op's because they are instructions that does nothing
// or has no effect when executed.
// They only affect the types and not the values.

fn traverse_noop(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[SymbolicExpression],
) -> Result<(), GeneratorError> {
    generator.traverse_args(builder, args)?;

    Ok(())
}

#[derive(Debug)]
pub struct ToInt;

impl Word for ToInt {
    fn name(&self) -> ClarityName {
        "to-int".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if let SymbolicExpressionType::LiteralValue(Value::UInt(val)) = args[0].expr {
            i128::try_from(val).map_err(|_| {
                GeneratorError::InternalError(
                    "Supplied argument is out of convertion range".to_string(),
                )
            })?;
        }

        traverse_noop(generator, builder, args)
    }
}

#[derive(Debug)]
pub struct ToUint;

impl Word for ToUint {
    fn name(&self) -> ClarityName {
        "to-uint".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if let SymbolicExpressionType::LiteralValue(Value::Int(val)) = args[0].expr {
            u128::try_from(val).map_err(|_| {
                GeneratorError::InternalError("Error converting a negative argument".to_string())
            })?;
        }

        traverse_noop(generator, builder, args)
    }
}

#[derive(Debug)]
pub struct ContractOf;

impl Word for ContractOf {
    fn name(&self) -> ClarityName {
        "contract-of".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_noop(generator, builder, args)
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::evaluate as eval;
    use crate::tools::TestEnvironment;
    use clarity::vm::{
        types::{PrincipalData, QualifiedContractIdentifier},
        Value,
    };

    #[test]
    #[should_panic(expected = "Supplied argument is out of convertion range")]
    fn to_int_big() {
        assert_eq!(
            eval("(to-int u340282366920938463463374607431768211455)"),
            Some(Value::Int(42))
        );
    }

    #[test]
    fn to_int() {
        assert_eq!(eval("(to-int u42)"), Some(Value::Int(42)));
    }

    #[test]
    #[should_panic(expected = "Error converting a negative argument")]
    fn to_uint_negative() {
        assert_eq!(eval("(to-uint -31)"), Some(Value::UInt(767)));
    }

    #[test]
    fn to_uint() {
        assert_eq!(eval("(to-uint 767)"), Some(Value::UInt(767)));
    }

    #[test]
    fn contract_of() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "clar2wasm-trait",
            r#"
(define-trait clar2wasm-trait
  ((add (int int) (response int int))))
(define-public (add (a int) (b int))
  (ok (+ a b)))
"#,
        );

        let val = env.init_contract_with_snippet(
            "contract-of",
            r#"
(use-trait clar2wasm-trait .clar2wasm-trait.clar2wasm-trait)
(define-public (test-contract-of (t <clar2wasm-trait>))
    (ok (contract-of t))) ;; Test subject: contract-of usage
(test-contract-of .clar2wasm-trait)
"#,
        );

        assert_eq!(
            val.unwrap(),
            Value::okay(Value::Principal(PrincipalData::Contract(
                QualifiedContractIdentifier::parse(
                    "S1G2081040G2081040G2081040G208105NK8PE5.clar2wasm-trait"
                )
                .unwrap()
            )))
            .unwrap()
        );
    }
}
