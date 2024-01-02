use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};

use super::{ComplexWord, SimpleWord};
use crate::wasm_generator::{GeneratorError, WasmGenerator};

// Functions below are considered no-op's because they are instructions that does nothing
// or has no effect when executed.
// They only affect the types and not the values.

#[derive(Debug)]
pub struct ToInt;

impl SimpleWord for ToInt {
    fn name(&self) -> ClarityName {
        "to-int".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let helper_func = generator.func_by_name("stdlib.to-int");
        builder.call(helper_func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct ToUint;

impl SimpleWord for ToUint {
    fn name(&self) -> ClarityName {
        "to-uint".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _arg_types: &[TypeSignature],
        _return_type: &TypeSignature,
    ) -> Result<(), GeneratorError> {
        let helper_func = generator.func_by_name("stdlib.to-uint");
        builder.call(helper_func);

        Ok(())
    }
}

#[derive(Debug)]
pub struct ContractOf;

impl ComplexWord for ContractOf {
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
        generator.traverse_args(builder, args)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::{PrincipalData, QualifiedContractIdentifier};
    use clarity::vm::Value;

    use crate::tools::{evaluate as eval, TestEnvironment};

    #[test]
    #[should_panic]
    fn to_int_out_of_range() {
        assert_eq!(
            eval("(to-int u170141183460469231731687303715884105728)"),
            Some(Value::Int(42))
        );
    }

    #[test]
    fn to_int_max_on_range() {
        assert_eq!(
            eval("(to-int u170141183460469231731687303715884105727)"),
            Some(Value::Int(170141183460469231731687303715884105727))
        );
    }

    #[test]
    fn to_int_zero() {
        assert_eq!(eval("(to-int u0)"), Some(Value::Int(0)));
    }

    #[test]
    fn to_int() {
        assert_eq!(eval("(to-int u42)"), Some(Value::Int(42)));
    }

    #[test]
    #[should_panic]
    fn to_uint_negative() {
        assert_eq!(eval("(to-uint -31)"), Some(Value::UInt(767)));
    }

    #[test]
    fn to_uint() {
        assert_eq!(eval("(to-uint 767)"), Some(Value::UInt(767)));
    }

    #[test]
    fn to_uint_zero() {
        assert_eq!(eval("(to-uint 0)"), Some(Value::UInt(0)));
    }

    #[test]
    fn contract_of() {
        let mut env = TestEnvironment::default();
        let _ = env.init_contract_with_snippet(
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
            Some(
                Value::okay(Value::Principal(PrincipalData::Contract(
                    QualifiedContractIdentifier::parse(
                        "S1G2081040G2081040G2081040G208105NK8PE5.clar2wasm-trait"
                    )
                    .unwrap()
                )))
                .unwrap()
            )
        );
    }
}
