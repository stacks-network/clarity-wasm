use clarity::vm::types::TypeSignature;
use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::{GlobalId, Module};

use super::ComplexWord;
use crate::error_mapping::ErrorMap;
use crate::wasm_generator::{
    add_placeholder_for_type, clar2wasm_ty, ArgumentsExt, GeneratorError, WasmGenerator,
};

fn get_global(module: &Module, name: &str) -> Result<GlobalId, GeneratorError> {
    module
        .globals
        .iter()
        .find(|global| {
            global
                .name
                .as_ref()
                .map_or(false, |other_name| name == other_name)
        })
        .map(|global| global.id())
        .ok_or_else(|| {
            GeneratorError::InternalError(format!("Expected to find a global named ${name}"))
        })
}

#[derive(Debug)]
pub struct ClaritySome;

impl ComplexWord for ClaritySome {
    fn name(&self) -> ClarityName {
        "some".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 1 {
            let (arg_name_offset_start, arg_name_len_expected) =
                generator.add_literal(&clarity::vm::Value::UInt(1))?;
            let (_, arg_name_len_got) =
                generator.add_literal(&clarity::vm::Value::UInt(args.len() as u128))?;
            builder
                .i32_const(arg_name_offset_start as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-offset")?)
                .i32_const((arg_name_len_expected + arg_name_len_got) as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-len")?)
                .i32_const(ErrorMap::ArgumentCountMismatch as i32)
                .call(generator.func_by_name("stdlib.runtime-error"));
        };

        let value = args.get_expr(0)?;
        // (some <val>) is represented by an i32 1, followed by the value
        builder.i32_const(1);

        if let TypeSignature::OptionalType(inner_type) = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("some expression must be typed".to_owned()))?
        {
            // WORKKAROUND: set inner value full type
            generator.set_expr_type(value, *inner_type.clone())?;

            generator.traverse_expr(builder, value)
        } else {
            Err(GeneratorError::TypeError(
                "expected optional type".to_owned(),
            ))
        }
    }
}

#[derive(Debug)]
pub struct ClarityOk;

impl ComplexWord for ClarityOk {
    fn name(&self) -> ClarityName {
        "ok".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 1 {
            let (arg_name_offset_start, arg_name_len_expected) =
                generator.add_literal(&clarity::vm::Value::UInt(1))?;
            let (_, arg_name_len_got) =
                generator.add_literal(&clarity::vm::Value::UInt(args.len() as u128))?;
            builder
                .i32_const(arg_name_offset_start as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-offset")?)
                .i32_const((arg_name_len_expected + arg_name_len_got) as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-len")?)
                .i32_const(ErrorMap::ArgumentCountMismatch as i32)
                .call(generator.func_by_name("stdlib.runtime-error"));
        };

        let value = args.get_expr(0)?;

        let TypeSignature::ResponseType(inner_types) = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("ok expression must be typed".to_owned()))?
            .clone()
        else {
            return Err(GeneratorError::TypeError(
                "expected response type".to_owned(),
            ));
        };

        // (ok <val>) is represented by an i32 1, followed by the ok value,
        // followed by a placeholder for the err value
        builder.i32_const(1);

        //WORKAROUND: set full type to ok value
        generator.set_expr_type(value, inner_types.0)?;
        generator.traverse_expr(builder, value)?;

        // deal with err placeholders
        let err_types = clar2wasm_ty(&inner_types.1);
        for err_type in err_types.iter() {
            add_placeholder_for_type(builder, *err_type);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ClarityErr;

impl ComplexWord for ClarityErr {
    fn name(&self) -> ClarityName {
        "err".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        if args.len() != 1 {
            let (arg_name_offset_start, arg_name_len_expected) =
                generator.add_literal(&clarity::vm::Value::UInt(1))?;
            let (_, arg_name_len_got) =
                generator.add_literal(&clarity::vm::Value::UInt(args.len() as u128))?;
            builder
                .i32_const(arg_name_offset_start as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-offset")?)
                .i32_const((arg_name_len_expected + arg_name_len_got) as i32)
                .global_set(get_global(&generator.module, "runtime-error-arg-len")?)
                .i32_const(ErrorMap::ArgumentCountMismatch as i32)
                .call(generator.func_by_name("stdlib.runtime-error"));
        };

        let value = args.get_expr(0)?;
        // (err <val>) is represented by an i32 0, followed by a placeholder
        // for the ok value, followed by the err value
        builder.i32_const(0);
        let ty = generator
            .get_expr_type(expr)
            .ok_or_else(|| GeneratorError::TypeError("err expression must be typed".to_owned()))?;
        if let TypeSignature::ResponseType(inner_types) = ty {
            let ok_types = clar2wasm_ty(&inner_types.0);
            for ok_type in ok_types.iter() {
                add_placeholder_for_type(builder, *ok_type);
            }
            // WORKAROUND: set full type to err value
            generator.set_expr_type(value, inner_types.1.clone())?
        } else {
            return Err(GeneratorError::TypeError(
                "expected response type".to_owned(),
            ));
        }
        generator.traverse_expr(builder, value)
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::evaluate;

    #[test]
    fn some_less_than_one_arg() {
        let result = evaluate("(some)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 0"));
    }

    #[test]
    fn some_more_than_one_arg() {
        let result = evaluate("(some 1 2)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 2"));
    }

    #[test]
    fn ok_less_than_one_arg() {
        let result = evaluate("(ok)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 0"));
    }

    #[test]
    fn ok_more_than_one_arg() {
        let result = evaluate("(ok 1 2)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 2"));
    }

    #[test]
    fn err_less_than_one_arg() {
        let result = evaluate("(err)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 0"));
    }

    #[test]
    fn err_more_than_one_arg() {
        let result = evaluate("(err 1 2)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expecting 1 arguments, got 2"));
    }
}
