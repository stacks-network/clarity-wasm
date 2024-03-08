use clarity::vm::errors::{Error, WasmError};
use std::ops::AddAssign;
use wasmtime::{AsContextMut, Instance, Store};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct Cost {
    pub runtime: i64,
    pub read_length: i64,
    pub read_count: i64,
    pub write_length: i64,
    pub write_count: i64,
}

impl AddAssign for Cost {
    fn add_assign(&mut self, rhs: Self) {
        self.runtime += rhs.runtime;
        self.read_length += rhs.read_length;
        self.read_count += rhs.read_count;
        self.write_length += rhs.write_length;
        self.write_count += rhs.write_count;
    }
}

impl Cost {
    pub fn free() -> Self {
        Cost {
            ..Default::default()
        }
    }

    pub fn from_instance_store<T>(
        instance: Instance,
        mut store: &mut Store<T>,
    ) -> Result<Self, Error> {
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

        Ok(Cost {
            runtime,
            read_length,
            read_count,
            write_length,
            write_count,
        })
    }

    pub fn add_runtime_const(mut self, runtime: i64) -> Self {
        self.runtime += runtime;
        self
    }

    pub fn add_runtime_linear(mut self, n: usize, a: i64, b: i64) -> Self {
        self.runtime += n as i64 * a + b;
        self
    }

    pub fn add_read(mut self, n_bytes: usize) -> Self {
        self.read_count += 1;
        self.read_length += n_bytes as i64;
        self
    }

    pub fn add_write(mut self, n_bytes: usize) -> Self {
        self.write_count += 1;
        self.write_length += n_bytes as i64;
        self
    }
}
