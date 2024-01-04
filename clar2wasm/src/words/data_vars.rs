use clarity::vm::{ClarityName, SymbolicExpression};
use walrus::ValType;

use super::ComplexWord;
use crate::wasm_generator::{ArgumentsExt, GeneratorError, LiteralMemoryEntry, WasmGenerator};

#[derive(Debug)]
pub struct DefineDataVar;

impl ComplexWord for DefineDataVar {
    fn name(&self) -> ClarityName {
        "define-data-var".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let _data_type = args.get_expr(1)?;
        let initial = args.get_expr(2)?;

        // Store the identifier as a string literal in the memory
        let (name_offset, name_length) = generator.add_string_literal(name);

        // The initial value can be placed on the top of the memory, since at
        // the top-level, we have not set up the call stack yet.
        let ty = generator
            .get_expr_type(initial)
            .expect("initial value expression must be typed")
            .clone();
        let offset = generator.module.locals.add(ValType::I32);
        builder
            .i32_const(generator.literal_memory_end as i32)
            .local_set(offset);

        // Traverse the initial value for the data variable (result is on the
        // data stack)
        generator.traverse_expr(builder, initial)?;

        // Write the initial value to the memory, to be read by the host.
        let size = generator.write_to_memory(builder, offset, 0, &ty);

        // Increment the literal memory end
        // FIXME: These initial values do not need to be saved in the literal
        //        memory forever... we just need them once, when .top-level
        //        is called.
        generator.literal_memory_end += size;

        // Push the name onto the data stack
        builder
            .i32_const(name_offset as i32)
            .i32_const(name_length as i32);

        // Push the offset onto the data stack
        builder.local_get(offset);

        // Push the size onto the data stack
        builder.i32_const(size as i32);

        // Call the host interface function, `define_variable`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.define_variable")
                .expect("function not found"),
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetDataVar;

impl ComplexWord for SetDataVar {
    fn name(&self) -> ClarityName {
        "var-set".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;
        let value = args.get_expr(1)?;
        generator.traverse_expr(builder, value)?;

        // Get the offset and length for this identifier in the literal memory
        let id_offset = *generator
            .literal_memory_offset
            .get(&LiteralMemoryEntry::Ascii(name.as_str().into()))
            .expect("variable not found: {name}");
        let id_length = name.len();

        // Create space on the call stack to write the value
        let ty = generator
            .get_expr_type(value)
            .expect("var-set value expression must be typed")
            .clone();
        let (offset, size) = generator.create_call_stack_local(builder, &ty, true, false);

        // Write the value to the memory, to be read by the host
        generator.write_to_memory(builder, offset, 0, &ty);

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(size);

        // Call the host interface function, `set_variable`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.set_variable")
                .expect("function not found"),
        );

        // `var-set` always returns `true`
        builder.i32_const(1);

        Ok(())
    }
}

#[derive(Debug)]
pub struct GetDataVar;

impl ComplexWord for GetDataVar {
    fn name(&self) -> ClarityName {
        "var-get".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let name = args.get_name(0)?;

        // Get the offset and length for this identifier in the literal memory
        let id_offset = *generator
            .literal_memory_offset
            .get(&LiteralMemoryEntry::Ascii(name.as_str().into()))
            .expect("variable not found: {name}");
        let id_length = name.len();

        // Create a new local to hold the result on the call stack
        let ty = generator
            .get_expr_type(expr)
            .expect("var-get expression must be typed")
            .clone();
        let (offset, size) = generator.create_call_stack_local(builder, &ty, true, true);

        // Push the identifier offset and length onto the data stack
        builder
            .i32_const(id_offset as i32)
            .i32_const(id_length as i32);

        // Push the offset and size to the data stack
        builder.local_get(offset).i32_const(size);

        // Call the host interface function, `get_variable`
        builder.call(
            generator
                .module
                .funcs
                .by_name("stdlib.get_variable")
                .expect("function not found"),
        );

        // Host interface fills the result into the specified memory. Read it
        // back out, and place the value on the data stack.
        generator.read_from_memory(builder, offset, 0, &ty);

        Ok(())
    }
}
