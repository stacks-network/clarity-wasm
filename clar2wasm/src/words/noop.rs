use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{ClarityName, SymbolicExpression};

use super::Word;

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
    use clarity::vm::{
        types::{PrincipalData, QualifiedContractIdentifier},
        Value,
    };

    use crate::tools::TestEnvironment;

    #[test]
    fn contract_of_eval() {
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
