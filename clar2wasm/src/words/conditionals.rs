use crate::wasm_generator::{clar2wasm_ty, ArgumentsExt};
use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{
    clarity_wasm::get_type_in_memory_size, ClarityName, SymbolicExpression, SymbolicExpressionType,
};
use walrus::{
    ir::{IfElse, InstrSeqType},
    InstrSeqBuilder, ValType,
};

use super::Word;

#[derive(Debug)]
pub struct If;

impl Word for If {
    fn name(&self) -> ClarityName {
        "if".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let conditional = args.get_expr(0)?;
        let true_branch = args.get_expr(1)?;
        let false_branch = args.get_expr(2)?;

        let return_type = clar2wasm_ty(
            generator
                .get_expr_type(expr)
                .expect("If results must be typed"),
        );

        // create block for true branch

        let mut true_block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &return_type,
        ));

        generator.traverse_expr(&mut true_block, true_branch)?;

        let id_true = true_block.id();

        let mut false_block = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &return_type,
        ));

        generator.traverse_expr(&mut false_block, false_branch)?;

        let id_false = false_block.id();

        generator.traverse_expr(builder, conditional)?;

        builder.instr(IfElse {
            consequent: id_true,
            alternative: id_false,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::evaluate;
    use clarity::vm::Value;

    #[test]
    fn trivial() {
        assert_eq!(evaluate("true"), Some(Value::Bool(true)));
    }

    #[test]
    fn what_if() {
        assert_eq!(evaluate("(if true true false)"), Some(Value::Bool(true)));
    }

    #[test]
    fn what_if_complex() {
        assert_eq!(evaluate("(if true (+ 1 1) (+ 2 2))"), Some(Value::Int(2)));
        assert_eq!(evaluate("(if false (+ 1 1) (+ 2 2))"), Some(Value::Int(4)));
    }

    #[test]
    fn what_if_extensive_condition() {
        assert_eq!(
            evaluate("(if (> 9001 9000) (+ 1 1) (+ 2 2))"),
            Some(Value::Int(2))
        );
    }
}
