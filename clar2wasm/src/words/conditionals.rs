use clarity::vm::types::{SequenceSubtype, TypeSignature};
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ir::{self, InstrSeqType};
use walrus::ValType;

use super::ComplexWord;
use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError,
    WasmGenerator,
};

#[derive(Debug)]
pub struct If;

impl ComplexWord for If {
    fn name(&self) -> ClarityName {
        "if".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let conditional = args.get_expr(0)?;
        let true_branch = args.get_expr(1)?;
        let false_branch = args.get_expr(2)?;

        let id_true = generator.block_from_expr(builder, true_branch)?;
        let id_false = generator.block_from_expr(builder, false_branch)?;

        generator.traverse_expr(builder, conditional)?;

        builder.instr(ir::IfElse {
            consequent: id_true,
            alternative: id_false,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Match;

impl ComplexWord for Match {
    fn name(&self) -> ClarityName {
        "match".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let match_on = args.get_expr(0)?;
        let success_binding = args.get_name(1)?;
        let success_body = args.get_expr(2)?;

        // save the current set of named locals, for later restoration
        let saved_bindings = generator.bindings.clone();

        generator.traverse_expr(builder, match_on)?;

        match generator.get_expr_type(match_on).cloned() {
            Some(TypeSignature::OptionalType(inner_type)) => {
                let none_body = args.get_expr(3)?;
                let some_locals = generator.save_to_locals(builder, &inner_type, true);

                generator
                    .bindings
                    .insert(success_binding.as_str().into(), some_locals);
                let some_block = generator.block_from_expr(builder, success_body)?;

                // we can restore early, since the none branch does not bind anything
                generator.bindings = saved_bindings;

                let none_block = generator.block_from_expr(builder, none_body)?;

                builder.instr(ir::IfElse {
                    consequent: some_block,
                    alternative: none_block,
                });

                Ok(())
            }
            Some(TypeSignature::ResponseType(inner_types)) => {
                let (ok_ty, err_ty) = &*inner_types;

                let err_binding = args.get_name(3)?;
                let err_body = args.get_expr(4)?;

                let err_locals = generator.save_to_locals(builder, err_ty, true);
                let ok_locals = generator.save_to_locals(builder, ok_ty, true);

                generator
                    .bindings
                    .insert(success_binding.as_str().into(), ok_locals);
                let ok_block = generator.block_from_expr(builder, success_body)?;

                // restore named locals
                generator.bindings = saved_bindings.clone();

                // bind err branch local
                generator
                    .bindings
                    .insert(err_binding.as_str().into(), err_locals);

                let err_block = generator.block_from_expr(builder, err_body)?;

                // restore named locals again
                generator.bindings = saved_bindings;

                builder.instr(ir::IfElse {
                    consequent: ok_block,
                    alternative: err_block,
                });

                Ok(())
            }
            _ => Err(GeneratorError::TypeError("Invalid type for match".into())),
        }
    }
}

#[derive(Debug)]
pub struct Filter;

impl ComplexWord for Filter {
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

        let memory = generator.get_memory();

        builder.loop_(None, |loop_| {
            let loop_id = loop_.id();

            // Load an element from the sequence
            let elem_size = generator.read_from_memory(loop_, input_offset, 0, elem_ty);

            // Stack now contains the value read from memory, note that this can be multiple values in case of
            // sequences.

            // [ Value ]

            // call the discriminator
            loop_.call(generator.func_by_name(discriminator.as_str()));

            // [ Discriminator result (bool) ]

            let mut success_branch = loop_.dangling_instr_seq(None);
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

            let fail_branch = loop_.dangling_instr_seq(None);
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

fn traverse_short_circuiting_list(
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

impl ComplexWord for And {
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
        traverse_short_circuiting_list(generator, builder, args, false)
    }
}

#[derive(Debug)]
pub struct Or;

impl ComplexWord for Or {
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
        traverse_short_circuiting_list(generator, builder, args, true)
    }
}

#[derive(Debug)]
pub struct Unwrap;

impl ComplexWord for Unwrap {
    fn name(&self) -> ClarityName {
        "unwrap!".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let input = args.get_expr(0)?;
        let throw = args.get_expr(1)?;

        generator.traverse_expr(builder, input)?;

        let throw_type = clar2wasm_ty(generator.get_expr_type(throw).expect("Throw must be typed"));

        let inner_type = match generator.get_expr_type(input) {
            Some(TypeSignature::OptionalType(inner_type)) => (**inner_type).clone(),
            Some(TypeSignature::ResponseType(inner_types)) => {
                let (ok_type, err_type) = &**inner_types;
                // Drop the err value;
                drop_value(builder, err_type);
                ok_type.clone()
            }
            _ => return Err(GeneratorError::TypeError("Invalid type for unwrap".into())),
        };

        // stack [ discriminant some_val ]
        let some_locals = generator.save_to_locals(builder, &inner_type, true);

        let mut throw_branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &throw_type,
        ));

        // The type-checker does not fill in the complete type for the throw
        // expression, so we need to manually update it here. If the return
        // type is not set, then we are not in a function, and the type can't
        // be determined.
        if let Some(return_ty) = &generator.return_type {
            generator.set_expr_type(throw, return_ty.clone());
        }
        generator.traverse_expr(&mut throw_branch, throw)?;
        generator.return_early(&mut throw_branch)?;

        let throw_branch_id = throw_branch.id();

        // stack [ discriminant ]

        let mut unwrap_branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &clar2wasm_ty(&inner_type),
        ));

        // in unwrap we restore the value from the locals
        for local in some_locals {
            unwrap_branch.local_get(local);
        }

        let unwrap_branch_id = unwrap_branch.id();

        builder.instr(ir::IfElse {
            consequent: unwrap_branch_id,
            alternative: throw_branch_id,
        });
        Ok(())
    }
}

#[derive(Debug)]
pub struct UnwrapErr;

impl ComplexWord for UnwrapErr {
    fn name(&self) -> ClarityName {
        "unwrap-err!".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let input = args.get_expr(0)?;
        let throw = args.get_expr(1)?;

        generator.traverse_expr(builder, input)?;

        let throw_type = clar2wasm_ty(generator.get_expr_type(throw).expect("Throw must be typed"));

        let (ok_type, err_type) = if let Some(TypeSignature::ResponseType(inner_types)) =
            generator.get_expr_type(input)
        {
            (**inner_types).clone()
        } else {
            return Err(GeneratorError::InternalError(
                "unwrap-error! only accepts response types".to_string(),
            ));
        };

        // Save the err value
        let err_locals = generator.save_to_locals(builder, &err_type, true);

        // drop the ok value
        drop_value(builder, &ok_type);

        let mut throw_branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &throw_type,
        ));

        // The type-checker does not fill in the complete type for the throw
        // expression, so we need to manually update it here. If the return
        // type is not set, then we are not in a function, and the type can't
        // be determined.
        if let Some(return_ty) = &generator.return_type {
            generator.set_expr_type(throw, return_ty.clone());
        }
        generator.traverse_expr(&mut throw_branch, throw)?;
        generator.return_early(&mut throw_branch)?;

        let throw_branch_id = throw_branch.id();

        // stack [ discriminant ]

        let mut unwrap_branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &clar2wasm_ty(&err_type),
        ));

        // in unwrap we restore the value from the locals
        for local in err_locals {
            unwrap_branch.local_get(local);
        }

        let unwrap_branch_id = unwrap_branch.id();

        builder
            // invert the value
            .i32_const(0)
            .binop(ir::BinaryOp::I32Eq)
            // conditionally branch
            .instr(ir::IfElse {
                consequent: unwrap_branch_id,
                alternative: throw_branch_id,
            });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Asserts;

impl ComplexWord for Asserts {
    fn name(&self) -> ClarityName {
        "asserts!".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let input = args.get_expr(0)?;
        let throw = args.get_expr(1)?;

        generator.traverse_expr(builder, input)?;

        let input_type = clar2wasm_ty(generator.get_expr_type(input).expect("Input must be typed"));
        let throw_type = clar2wasm_ty(generator.get_expr_type(throw).expect("Throw must be typed"));

        let mut success_branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &input_type,
        ));

        // always returns true
        success_branch.i32_const(1);
        let succ_branch_id = success_branch.id();

        let mut throw_branch = builder.dangling_instr_seq(InstrSeqType::new(
            &mut generator.module.types,
            &[],
            &throw_type,
        ));

        // The type-checker does not fill in the complete type for the throw
        // expression, so we need to manually update it here. If the return
        // type is not set, then we are not in a function, and the type can't
        // be determined.
        if let Some(return_ty) = &generator.return_type {
            generator.set_expr_type(throw, return_ty.clone());
        }
        generator.traverse_expr(&mut throw_branch, throw)?;
        generator.return_early(&mut throw_branch)?;

        let throw_branch_id = throw_branch.id();

        // stack [ discriminant ]

        builder.instr(ir::IfElse {
            consequent: succ_branch_id,
            alternative: throw_branch_id,
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct Try;

impl ComplexWord for Try {
    fn name(&self) -> ClarityName {
        "try!".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let input = args.get_expr(0)?;
        generator.traverse_expr(builder, input)?;

        let (succ_branch_id, throw_branch_id) = match generator.get_expr_type(input).cloned() {
            Some(ref full_type @ TypeSignature::OptionalType(ref inner_type)) => {
                let some_type = &**inner_type;

                let some_locals = generator.save_to_locals(builder, some_type, true);

                let mut throw_branch = builder.dangling_instr_seq(InstrSeqType::new(
                    &mut generator.module.types,
                    &[],
                    &clar2wasm_ty(full_type),
                ));

                // in the case of throw, we need to restore the value, and re-push the discriminant
                throw_branch.i32_const(0);
                add_placeholder_for_clarity_type(&mut throw_branch, some_type);
                generator.return_early(&mut throw_branch)?;

                let throw_branch_id = throw_branch.id();

                // on Some

                let mut succ_branch = builder.dangling_instr_seq(InstrSeqType::new(
                    &mut generator.module.types,
                    &[],
                    &clar2wasm_ty(some_type),
                ));

                // in unwrap we restore the value from the locals
                for local in &some_locals {
                    succ_branch.local_get(*local);
                }

                let succ_branch_id = succ_branch.id();

                (succ_branch_id, throw_branch_id)
            }
            Some(ref full_type @ TypeSignature::ResponseType(ref inner_types)) => {
                let (ok_type, err_type) = &**inner_types;

                // save both values to local
                let err_locals = generator.save_to_locals(builder, err_type, true);
                let ok_locals = generator.save_to_locals(builder, ok_type, true);

                let mut throw_branch = builder.dangling_instr_seq(InstrSeqType::new(
                    &mut generator.module.types,
                    &[],
                    &clar2wasm_ty(full_type),
                ));

                // in the case of throw, we need to re-push the discriminant, and restore both values
                throw_branch.i32_const(0);
                add_placeholder_for_clarity_type(&mut throw_branch, ok_type);
                for local in &err_locals {
                    throw_branch.local_get(*local);
                }
                generator.return_early(&mut throw_branch)?;

                let throw_branch_id = throw_branch.id();

                // On success

                let mut succ_branch = builder.dangling_instr_seq(InstrSeqType::new(
                    &mut generator.module.types,
                    &[],
                    &clar2wasm_ty(ok_type),
                ));

                // in ok case we restore the value from the locals
                for local in &ok_locals {
                    succ_branch.local_get(*local);
                }

                let succ_branch_id = succ_branch.id();

                (succ_branch_id, throw_branch_id)
            }
            _ => return Err(GeneratorError::TypeError("Invalid type for unwrap".into())),
        };

        // stack [ discriminant ]

        builder.instr(ir::IfElse {
            consequent: succ_branch_id,
            alternative: throw_branch_id,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clarity::vm::Value;

    use crate::tools::{evaluate as eval, TestEnvironment};

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

    #[test]
    fn clar_match_a() {
        const ADD_10: &str = "
(define-private (add-10 (x (response int int)))
 (match x
   val (+ val 10)
   err (+ err 107)))";

        assert_eq!(
            eval(&format!("{ADD_10} (add-10 (ok 115))")),
            Some(Value::Int(125))
        );
        assert_eq!(
            eval(&format!("{ADD_10} (add-10 (err 18))")),
            Some(Value::Int(125))
        );
    }

    #[test]
    fn clar_match_b() {
        const ADD_10: &str = "
(define-private (add-10 (x (optional int)))
 (match x
   val val
   1001))";

        assert_eq!(
            eval(&format!("{ADD_10} (add-10 none)")),
            Some(Value::Int(1001))
        );

        assert_eq!(
            eval(&format!("{ADD_10} (add-10 (some 10))")),
            Some(Value::Int(10))
        );
    }

    #[test]
    fn unwrap_a() {
        const FN: &str = "
(define-private (unwrapper (x (optional int)))
  (+ (unwrap! x 23) 10))";

        assert_eq!(
            eval(&format!("{FN} (unwrapper none)")),
            Some(Value::Int(23))
        );

        assert_eq!(
            eval(&format!("{FN} (unwrapper (some 10))")),
            Some(Value::Int(20))
        );
    }

    #[test]
    fn unwrap_b() {
        const FN: &str = "
(define-private (unwrapper (x (response int int)))
  (+ (unwrap! x 23) 10))";

        assert_eq!(
            eval(&format!("{FN} (unwrapper (err 9999))")),
            Some(Value::Int(23))
        );

        assert_eq!(
            eval(&format!("{FN} (unwrapper (ok 10))")),
            Some(Value::Int(20))
        );
    }

    #[test]
    fn unwrap_err() {
        const FN: &str = "
(define-private (unwrapper (x (response int int)))
  (+ (unwrap-err! x 23) 10))";

        assert_eq!(
            eval(&format!("{FN} (unwrapper (err 9999))")),
            Some(Value::Int(10009))
        );

        assert_eq!(
            eval(&format!("{FN} (unwrapper (ok 10))")),
            Some(Value::Int(23))
        );
    }

    /// Verify that the full response type is set correctly for the throw
    /// expression.
    #[test]
    fn response_type_bug() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "snippet",
            r#"
(define-private (foo)
    (ok u1)
)
(define-read-only (get-count-at-block (block uint))
    (ok (unwrap! (foo) (err u100)))
)
            "#,
        )
        .unwrap();
    }

    /// Verify that the full response type is set correctly for the throw
    /// expression.
    #[test]
    fn response_type_err_bug() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet(
            "snippet",
            r#"
(define-private (foo)
    (err u1)
)

(define-read-only (get-count-at-block (block uint))
    (ok (unwrap-err! (foo) (err u100)))
)
            "#,
        )
        .unwrap();
    }

    const TRY_FN: &str = "
(define-private (tryhard (x (response int int)))
  (ok (+ (try! x) 10)))";

    #[test]
    fn try_a() {
        assert_eq!(eval(&format!("{TRY_FN} (tryhard (ok 1))")), eval("(ok 11)"),);
    }

    #[test]
    fn try_b() {
        assert_eq!(
            eval(&format!("{TRY_FN} (tryhard (err 1))")),
            eval("(err 1)"),
        );
    }

    const TRY_FN_OPT: &str = "
(define-private (tryharder (x (optional int)))
  (some (+ (try! x) 10)))";

    #[test]
    fn try_c() {
        assert_eq!(
            eval(&format!("{TRY_FN_OPT} (tryharder (some 1))")),
            eval("(some 11)"),
        );
    }

    #[test]
    fn try_d() {
        assert_eq!(
            eval(&format!("{TRY_FN_OPT} (tryharder none)")),
            Some(Value::none())
        );
    }

    const ASSERT: &str = "
      (define-private (is-even (x int))
        (is-eq (* (/ x 2) 2) x))

      (define-private (assert-even (x int))
        (begin
          (asserts! (is-even x) (+ x 10))
          99))
    ";

    #[test]
    fn asserts_a() {
        assert_eq!(
            eval(&format!("{ASSERT} (assert-even 2)")),
            Some(Value::Int(99))
        );
    }

    #[test]
    fn asserts_b() {
        assert_eq!(
            eval(&format!("{ASSERT} (assert-even 1)")),
            Some(Value::Int(11))
        );
    }

    #[test]
    fn asserts_top_level_true() {
        assert_eq!(eval("(asserts! true (err u1))"), Some(Value::Bool(true)));
    }

    #[test]
    fn asserts_top_level_false() {
        let mut env = TestEnvironment::default();
        env.init_contract_with_snippet("snippet", "(asserts! false (err u1))")
            .expect_err("should panic");
    }
}
