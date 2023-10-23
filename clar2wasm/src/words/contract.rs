use crate::wasm_generator::{ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{
    types::PrincipalData, ClarityName, SymbolicExpression, SymbolicExpressionType, Value,
};
use walrus::ValType;

use super::Word;

#[derive(Debug)]
pub struct AsContract;

impl Word for AsContract {
    fn name(&self) -> ClarityName {
        "as-contract".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let inner = args.get_expr(0)?;

        // Call the host interface function, `enter_as_contract`
        builder.call(generator.func_by_name("enter_as_contract"));

        // Traverse the inner expression
        generator.traverse_expr(builder, inner)?;

        // Call the host interface function, `exit_as_contract`
        builder.call(generator.func_by_name("exit_as_contract"));

        Ok(())
    }
}

#[derive(Debug)]
pub struct ContractCall;

impl Word for ContractCall {
    fn name(&self) -> ClarityName {
        "contract-call?".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let function_name = args.get_name(1)?;
        let SymbolicExpressionType::LiteralValue(Value::Principal(PrincipalData::Contract(
            ref contract_identifier,
        ))) = args.get_expr(0)?.expr
        else {
            todo!("dynamic contract calls are not yet supported")
        };

        // shadow args
        let args = if args.len() >= 2 { &args[2..] } else { &[] };

        // Push the contract identifier onto the stack
        // TODO(#111): These should be tracked for reuse, similar to the string literals
        let (id_offset, id_length) = generator.add_literal(&contract_identifier.clone().into());
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the function name onto the stack
        let (fn_offset, fn_length) = generator.add_identifier_string_literal(function_name);
        builder
            .i32_const(fn_offset as i32)
            .i32_const(fn_length as i32);

        // Write the arguments to the call stack, to be read by the host
        let arg_offset = generator.module.locals.add(ValType::I32);
        builder
            .global_get(generator.stack_pointer)
            .local_set(arg_offset);
        let mut arg_length = 0;
        for arg in args {
            // Traverse the argument, pushing it onto the stack
            generator.traverse_expr(builder, arg)?;

            let arg_ty = generator
                .get_expr_type(arg)
                .expect("contract-call? argument must be typed")
                .clone();

            arg_length += generator.write_to_memory(builder, arg_offset, arg_length, &arg_ty);
        }

        // Push the arguments offset and length onto the data stack
        builder.local_get(arg_offset).i32_const(arg_length as i32);

        // Reserve space for the return value
        let return_ty = generator
            .get_expr_type(expr)
            .expect("contract-call? expression must be typed")
            .clone();
        let (return_offset, return_size) = generator.create_call_stack_local(
            builder,
            generator.stack_pointer,
            &return_ty,
            true,
            true,
        );

        // Push the return offset and size to the data stack
        builder.local_get(return_offset).i32_const(return_size);

        // Call the host interface function, `static_contract_call`
        builder.call(generator.func_by_name("static_contract_call"));

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, return_offset, 0, &return_ty);

        Ok(())
    }
}
