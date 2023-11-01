use crate::wasm_generator::{
    add_placeholder_for_clarity_type, clar2wasm_ty, drop_value, ArgumentsExt, GeneratorError,
    WasmGenerator,
};
use clarity::vm::{types::TypeSignature, ClarityName, SymbolicExpression};
use walrus::ir::InstrSeqType;

use super::Word;

#[derive(Debug)]
pub struct DefaultTo;

impl Word for DefaultTo {
    fn name(&self) -> ClarityName {
        "default-to".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        // There are a `default` value and an `optional` arguments.
        // (default-to 767 (some 1))
        // i64              i64               i32        i64           i64
        // default-val-low, default-val-high, indicator, plc-val-low, plc-val-high
        let default = args.get_expr(0)?;
        let optional = args.get_expr(1)?;

        generator.traverse_args(builder, args)?;

        // Default value type
        let default_ty = generator
            .get_expr_type(default)
            .expect("default expression must be typed")
            .clone();

        // Optional value type
        let opt_ty = generator
            .get_expr_type(optional)
            .expect("optional expression must be typed")
            .clone();
        // Optional value
        let opt_val_ty = if let TypeSignature::OptionalType(opt_type) = &opt_ty {
            &**opt_type
        } else {
            return Err(GeneratorError::TypeError(format!(
                "Expected an Optional type. Found {:?}",
                opt_ty
            )));
        };
        // Save Optional value to locals
        let opt_val_locals = generator.save_to_locals(builder, opt_val_ty, true);

        // Params and result types for the if_else branch
        let out_types = clar2wasm_ty(&default_ty);

        builder.if_else(
            InstrSeqType::new(&mut generator.module.types, &out_types, &out_types),
            |then| {
                drop_value(then, &default_ty);
                if &default_ty != opt_val_ty {
                    add_placeholder_for_clarity_type(then, &default_ty);
                } else {
                    for opt_val_local in opt_val_locals {
                        then.local_get(opt_val_local);
                    }
                }
            },
            |_| {},
        );

        Ok(())
    }
}
