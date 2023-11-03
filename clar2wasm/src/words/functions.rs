use std::collections::HashMap;

use crate::wasm_generator::{clar2wasm_ty, ArgumentsExt, GeneratorError, WasmGenerator};
use clarity::vm::{representations::Span, types::FunctionType, ClarityName, SymbolicExpression};
use walrus::{
    ir::{Block, InstrSeqType},
    FunctionBuilder, FunctionId, InstrSeqBuilder, ValType,
};

use super::Word;

#[derive(Clone)]
pub struct TypedVar<'a> {
    pub name: &'a ClarityName,
    pub type_expr: &'a SymbolicExpression,
    pub decl_span: Span,
}

enum FunctionKind {
    Public,
    Private,
    ReadOnly,
}

fn traverse_define_function(
    generator: &mut WasmGenerator,
    builder: &mut InstrSeqBuilder,
    name: &ClarityName,
    body: &SymbolicExpression,
    kind: FunctionKind,
) -> Result<FunctionId, GeneratorError> {
    let opt_function_type = match kind {
        FunctionKind::ReadOnly => {
            builder.i32_const(0);
            generator
                .contract_analysis
                .get_read_only_function_type(name.as_str())
        }
        FunctionKind::Public => {
            builder.i32_const(1);
            generator
                .contract_analysis
                .get_public_function_type(name.as_str())
        }
        FunctionKind::Private => {
            builder.i32_const(2);
            generator
                .contract_analysis
                .get_private_function(name.as_str())
        }
    };
    let function_type = if let Some(FunctionType::Fixed(fixed)) = opt_function_type {
        fixed.clone()
    } else {
        return Err(GeneratorError::InternalError(match opt_function_type {
            Some(_) => "expected fixed function type".to_string(),
            None => format!("unable to find function type for {}", name.as_str()),
        }));
    };

    // Call the host interface to save this function
    // Arguments are kind (already pushed) and name (offset, length)
    let (id_offset, id_length) = generator.add_string_literal(name);
    builder
        .i32_const(id_offset as i32)
        .i32_const(id_length as i32);

    // Call the host interface function, `define_function`
    builder.call(
        generator
            .module
            .funcs
            .by_name("define_function")
            .expect("define_function not found"),
    );

    let mut locals = HashMap::new();

    // Setup the parameters
    let mut param_locals = Vec::new();
    let mut params_types = Vec::new();
    for param in function_type.args.iter() {
        let param_types = clar2wasm_ty(&param.signature);
        let mut plocals = Vec::with_capacity(param_types.len());
        for ty in param_types {
            let local = generator.module.locals.add(ty);
            param_locals.push(local);
            plocals.push(local);
            params_types.push(ty);
        }
        locals.insert(param.name.to_string(), plocals.clone());
    }

    let results_types = clar2wasm_ty(&function_type.returns);
    let mut func_builder = FunctionBuilder::new(
        &mut generator.module.types,
        params_types.as_slice(),
        results_types.as_slice(),
    );
    func_builder.name(name.as_str().to_string());
    let mut func_body = func_builder.func_body();

    // Function prelude
    // Save the frame pointer in a local variable.
    let frame_pointer = generator.module.locals.add(ValType::I32);
    func_body
        .global_get(generator.stack_pointer)
        .local_set(frame_pointer);

    // Setup the locals map for this function, saving the top-level map to
    // restore after.
    let top_level_locals = std::mem::replace(&mut generator.locals, locals);

    let mut block = func_body.dangling_instr_seq(InstrSeqType::new(
        &mut generator.module.types,
        &[],
        results_types.as_slice(),
    ));
    let block_id = block.id();

    // Traverse the body of the function
    generator.traverse_expr(&mut block, body)?;

    // TODO: We need to ensure that all exits from the function go through
    // the postlude. Maybe put the body in a block, and then have any exits
    // from the block go to the postlude with a `br` instruction?

    // Insert the function body block into the function
    func_body.instr(Block { seq: block_id });

    // Function postlude
    // Restore the initial stack pointer.
    func_body
        .local_get(frame_pointer)
        .global_set(generator.stack_pointer);

    // Restore the top-level locals map.
    generator.locals = top_level_locals;

    Ok(func_builder.finish(param_locals, &mut generator.module.funcs))
}

#[derive(Debug)]
pub struct DefinePrivateFunction;

impl Word for DefinePrivateFunction {
    fn name(&self) -> ClarityName {
        "define-private".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        let body = args.get_expr(1)?;

        traverse_define_function(generator, builder, name, body, FunctionKind::Private)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefineReadonlyFunction;

impl Word for DefineReadonlyFunction {
    fn name(&self) -> ClarityName {
        "define-read-only".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        let body = args.get_expr(1)?;

        let function_id =
            traverse_define_function(generator, builder, name, body, FunctionKind::ReadOnly)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}

#[derive(Debug)]
pub struct DefinePublicFunction;

impl Word for DefinePublicFunction {
    fn name(&self) -> ClarityName {
        "define-public".into()
    }

    fn traverse(
        &self,
        generator: &mut WasmGenerator,
        builder: &mut walrus::InstrSeqBuilder,
        _expr: &SymbolicExpression,
        args: &[SymbolicExpression],
    ) -> Result<(), GeneratorError> {
        let Some(signature) = args.get_expr(0)?.match_list() else {
            return Err(GeneratorError::NotImplemented);
        };
        let name = signature.get_name(0)?;
        let body = args.get_expr(1)?;

        let function_id =
            traverse_define_function(generator, builder, name, body, FunctionKind::Public)?;
        generator.module.exports.add(name.as_str(), function_id);
        Ok(())
    }
}
