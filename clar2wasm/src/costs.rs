use clarity::vm::errors::{Error, WasmError};
use std::ops::AddAssign;
use walrus::{ir::BinaryOp, GlobalId, InstrSeqBuilder};
use wasmtime::{AsContextMut, Instance, Store};

#[derive(Debug)]
pub struct Cost {
    runtime: u64,
    read_length: u64,
    read_count: u64,
    write_length: u64,
    write_count: u64,

    runtime_cost_global: GlobalId,
    read_length_global: GlobalId,
    read_count_global: GlobalId,
    write_length_global: GlobalId,
    write_count_global: GlobalId,
}

impl Cost {
    pub fn new(
        runtime_cost_global: GlobalId,
        read_length_global: GlobalId,
        read_count_global: GlobalId,
        write_length_global: GlobalId,
        write_count_global: GlobalId,
    ) -> Self {
        Cost {
            runtime_cost_global,
            read_length_global,
            read_count_global,
            write_length_global,
            write_count_global,

            runtime: 0,
            read_length: 0,
            read_count: 0,
            write_length: 0,
            write_count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.runtime = 0;
        self.read_length = 0;
        self.read_count = 0;
        self.write_length = 0;
        self.write_count = 0;
    }

    pub fn const_runtime(&mut self, runtime: u64) {
        self.runtime += runtime;
    }

    pub fn const_runtime_linear(&mut self, n: usize, a: u64, b: u64) {
        self.runtime += n as u64 * a + b;
    }

    pub fn add_read(&mut self, n_bytes: usize) {
        self.read_count += 1;
        self.read_length += n_bytes as u64;
    }

    pub fn add_write(&mut self, n_bytes: usize) {
        self.write_count += 1;
        self.write_length += n_bytes as u64;
    }

    pub fn emit(&mut self, builder: &mut InstrSeqBuilder) {
        println!("doing emit ok {}", self.runtime);
        if self.runtime > 0 {
            builder.global_get(self.runtime_cost_global);
            builder.i64_const(self.runtime as i64);
            builder.binop(BinaryOp::I64Add);
            builder.global_set(self.runtime_cost_global);
        }
        if self.read_length > 0 {
            builder.global_get(self.read_length_global);
            builder.i64_const(self.read_length as i64);
            builder.binop(BinaryOp::I64Add);
            builder.global_set(self.read_length_global);
        }
        if self.read_count > 0 {
            builder.global_get(self.read_count_global);
            builder.i64_const(self.read_count as i64);
            builder.binop(BinaryOp::I64Add);
            builder.global_set(self.read_count_global);
        }
        if self.write_length > 0 {
            builder.global_get(self.write_length_global);
            builder.i64_const(self.write_length as i64);
            builder.binop(BinaryOp::I64Add);
            builder.global_set(self.write_length_global);
        }
        if self.write_count > 0 {
            builder.global_get(self.write_count_global);
            builder.i64_const(self.write_count as i64);
            builder.binop(BinaryOp::I64Add);
            builder.global_set(self.write_count_global);
        }
        self.clear()
    }

    pub fn finalize<T>(
        self,
        instance: Instance,
        mut store: &mut Store<T>,
    ) -> Result<CostFinalized, Error> {
        let Some(wasmtime::Val::I64(runtime)) = instance
            .get_global(store.as_context_mut(), "cost-rt")
            .map(|cost| cost.get(&mut store))
        else {
            // TODO: use custom error
            return Err(Error::Wasm(WasmError::WasmGeneratorError(
                "No runtime cost global found".to_string(),
            )));
        };

        let Some(wasmtime::Val::I64(read_length)) = instance
            .get_global(store.as_context_mut(), "cost-rl")
            .map(|cost| cost.get(&mut store))
        else {
            // TODO: use custom error
            return Err(Error::Wasm(WasmError::WasmGeneratorError(
                "No read length global found".to_string(),
            )));
        };

        let Some(wasmtime::Val::I64(read_count)) = instance
            .get_global(store.as_context_mut(), "cost-rc")
            .map(|cost| cost.get(&mut store))
        else {
            // TODO: use custom error
            return Err(Error::Wasm(WasmError::WasmGeneratorError(
                "No read count global found".to_string(),
            )));
        };

        let Some(wasmtime::Val::I64(write_length)) = instance
            .get_global(store.as_context_mut(), "cost-wl")
            .map(|cost| cost.get(&mut store))
        else {
            // TODO: use custom error
            return Err(Error::Wasm(WasmError::WasmGeneratorError(
                "No write length global found".to_string(),
            )));
        };

        let Some(wasmtime::Val::I64(write_count)) = instance
            .get_global(store.as_context_mut(), "cost-wc")
            .map(|cost| cost.get(store))
        else {
            // TODO: use custom error
            return Err(Error::Wasm(WasmError::WasmGeneratorError(
                "No write count global found".to_string(),
            )));
        };

        Ok(CostFinalized {
            runtime: runtime as u64,
            read_length: read_length as u64,
            read_count: read_count as u64,
            write_length: write_length as u64,
            write_count: write_count as u64,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CostFinalized {
    runtime: u64,
    read_length: u64,
    read_count: u64,
    write_length: u64,
    write_count: u64,
}

impl AddAssign for CostFinalized {
    fn add_assign(&mut self, rhs: Self) {
        self.runtime += rhs.runtime;
        self.read_length += rhs.read_length;
        self.read_count += rhs.read_count;
        self.write_length += rhs.write_length;
        self.write_count += rhs.write_count;
    }
}
