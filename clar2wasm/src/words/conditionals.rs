use crate::wasm_generator::{clar2wasm_ty, ArgumentsExt};
use crate::wasm_generator::{GeneratorError, WasmGenerator};
use clarity::vm::{
    types::{SequenceSubtype, TypeSignature},
    ClarityName, SymbolicExpression,
};
use walrus::{
    ir::{self, InstrSeqType},
    ValType,
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

        builder.instr(ir::IfElse {
            consequent: id_true,
            alternative: id_false,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Filter;

impl Word for Filter {
    fn name(&self) -> ClarityName {
        "filter".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let discriminator = args.get_name(0)?;
        let sequence = args.get_expr(1)?;

        generator.traverse_expr(builder, sequence)?;

        // Get the type of the sequence
        let ty = generator
            .get_expr_type(sequence)
            .expect("sequence expression must be typed")
            .clone();

        // Get the type of the sequence
        let seq_ty = match &ty {
            TypeSignature::SequenceType(seq_ty) => seq_ty.clone(),
            _ => {
                return Err(GeneratorError::InternalError(
                    "expected sequence type".to_string(),
                ));
            }
        };

        // Setup neccesary locals for the operations.
        let input_len = generator.module.locals.add(ValType::I32);
        let input_offset = generator.module.locals.add(ValType::I32);
        let input_end = generator.module.locals.add(ValType::I32);
        let output_len = generator.module.locals.add(ValType::I32);

        let elem_ty = match &seq_ty {
            SequenceSubtype::ListType(list_type) => list_type.get_list_item_type(),
            _ => unimplemented!("Unsupported sequence type"),
        };

        builder
            // [ input_offset, input_len ]
            .local_set(input_len)
            // [ input_offset ]
            .local_tee(input_offset)
            // [ input_offset ]
            .local_get(input_len)
            // [ input_offset, input_len ]
            .binop(ir::BinaryOp::I32Add)
            // [ input_end ]
            .local_set(input_end);
        // [ ]
        // now we have an empty stack, and three initialized locals

        // reserve space for the length of the output list

        let (output_offset, _) = generator.create_call_stack_local(builder, &ty, false, true);

        // the loop returns nothing itself, but builds the result in the data stack
        let loop_body_ty = InstrSeqType::new(&mut generator.module.types, &[], &[]);
        let memory = generator.get_memory();

        builder.loop_(loop_body_ty, |loop_| {
            let loop_id = loop_.id();

            // Load an element from the sequence
            let elem_size = generator.read_from_memory(loop_, input_offset, 0, elem_ty);

            // Stack now contains the value read from memory, note that this can be multiple values in case of
            // sequences.

            // [ Value ]

            // call the discriminator
            loop_.call(generator.func_by_name(discriminator.as_str()));

            // [ Discriminator result (bool) ]

            let mut success_branch =
                loop_.dangling_instr_seq(InstrSeqType::new(&mut generator.module.types, &[], &[]));
            let succ_id = success_branch.id();

            // on success, increment length and copy value
            // memory.copy takes source, destination and size in push order
            // (reverse on stack)

            success_branch
                // []
                .local_get(output_offset)
                // [ output_ofs ]
                .local_get(output_len)
                // [ output_ofs, output_len ]
                .binop(ir::BinaryOp::I32Add)
                // [ output_write_pos ]
                .local_get(input_offset)
                // [ output_write_pos, input_offset ]
                .i32_const(elem_size)
                // [ output_write_pos, input_offset, element_size ]
                .memory_copy(memory, memory)
                // [  ]
                .local_get(output_len)
                // [ output_len ]
                .i32_const(elem_size)
                // [ output_len, elem_size ]
                .binop(ir::BinaryOp::I32Add)
                // [ new_output_len ]
                .local_set(output_len);
            // [  ]

            // fail branch is a no-op (FIXME there is most certainly a better way to do this)

            let fail_branch =
                loop_.dangling_instr_seq(InstrSeqType::new(&mut generator.module.types, &[], &[]));
            let fail_id = fail_branch.id();

            loop_.instr(ir::IfElse {
                consequent: succ_id,
                alternative: fail_id,
            });

            // increment offset, leaving the new offset on the stack for the end check
            loop_
                .local_get(input_offset)
                .i32_const(elem_size)
                .binop(ir::BinaryOp::I32Add)
                .local_tee(input_offset);

            // Loop if we haven't reached the end of the sequence
            loop_
                .local_get(input_end)
                .binop(ir::BinaryOp::I32LtU)
                .br_if(loop_id);
        });

        builder.local_get(output_offset);
        builder.local_get(output_len);

        Ok(())
    }
}

fn traverse_short_circuting_list(
    generator: &mut WasmGenerator,
    builder: &mut walrus::InstrSeqBuilder,
    args: &[SymbolicExpression],
    invert: bool,
) -> Result<(), GeneratorError> {
    let n_branches = args.len();

    let mut branches = vec![];

    let noop = builder
        .dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &[ValType::I32],
        ))
        // for now, the noop branch just adds a false to break out of the next iteration
        .i32_const(if invert { 1 } else { 0 })
        .id();

    for i in 0..n_branches {
        let branch_expr = args.get_expr(i)?;

        let mut branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &[ValType::I32],
        ));

        generator.traverse_expr(&mut branch, branch_expr)?;

        branches.push(branch.id());
    }

    builder.i32_const(if invert { 0 } else { 1 });

    for branch in branches {
        if invert {
            builder.instr(ir::IfElse {
                consequent: noop,
                alternative: branch,
            });
        } else {
            builder.instr(ir::IfElse {
                consequent: branch,
                alternative: noop,
            });
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct And;

impl Word for And {
    fn name(&self) -> ClarityName {
        "and".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_short_circuting_list(generator, builder, args, false)
    }
}

#[derive(Debug)]
pub struct Or;

impl Word for Or {
    fn name(&self) -> ClarityName {
        "or".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        traverse_short_circuting_list(generator, builder, args, true)
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::evaluate as eval;
    use clarity::vm::Value;

    #[test]
    fn trivial() {
        assert_eq!(eval("true"), Some(Value::Bool(true)));
    }

    #[test]
    fn what_if() {
        assert_eq!(eval("(if true true false)"), Some(Value::Bool(true)));
    }

    #[test]
    fn what_if_complex() {
        assert_eq!(eval("(if true (+ 1 1) (+ 2 2))"), Some(Value::Int(2)));
        assert_eq!(eval("(if false (+ 1 1) (+ 2 2))"), Some(Value::Int(4)));
    }

    #[test]
    fn what_if_extensive_condition() {
        assert_eq!(
            eval("(if (> 9001 9000) (+ 1 1) (+ 2 2))"),
            Some(Value::Int(2))
        );
    }

    #[test]
    fn filter() {
        assert_eq!(
            eval(
                "
(define-private (is-great (number int))
  (> number 2))

(filter is-great (list 1 2 3 4))
"
            ),
            eval("(list 3 4)"),
        );
    }

    #[test]
    fn and() {
        assert_eq!(
            eval(
                r#"
(define-data-var cursor int 6)
(and
  (var-set cursor (+ (var-get cursor) 1))
  true
  (var-set cursor (+ (var-get cursor) 1))
  false
  (var-set cursor (+ (var-get cursor) 1)))
(var-get cursor)
                "#
            ),
            eval("8")
        );
    }

    #[test]
    fn or() {
        assert_eq!(
            eval(
                r#"
(define-data-var cursor int 6)
(or
  (begin
    (var-set cursor (+ (var-get cursor) 1))
    false)
  false
  (var-set cursor (+ (var-get cursor) 1))
  (var-set cursor (+ (var-get cursor) 1)))
(var-get cursor)
                "#
            ),
            eval("8")
        );
    }
}
