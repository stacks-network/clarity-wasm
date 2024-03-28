use clarity::vm::errors::WasmError;

#[derive(Clone, Default, Debug)]
pub struct WasmCost {
    runtime: u64,
    read_count: u64,
    read_length: u64,
    write_count: u64,
    write_length: u64,
}

impl WasmCost {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn max() -> Self {
        WasmCost {
            runtime: u64::MAX,
            read_count: u64::MAX,
            read_length: u64::MAX,
            write_count: u64::MAX,
            write_length: u64::MAX,
        }
    }

    pub fn runtime(&self) -> u64 {
        self.runtime
    }

    pub fn set_runtime(mut self, runtime: u64) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn set_read_count(mut self, read_count: u64) -> Self {
        self.read_count = read_count;
        self
    }

    pub fn set_read_length(mut self, read_length: u64) -> Self {
        self.read_length = read_length;
        self
    }

    pub fn set_write_count(mut self, write_count: u64) -> Self {
        self.write_count = write_count.into();
        self
    }

    pub fn set_write_length(mut self, write_length: u64) -> Self {
        self.write_length = write_length;
        self
    }

    pub fn deduct(&mut self, cost: WasmCost) -> Result<(), WasmError> {
        self.runtime = self
            .runtime
            .checked_sub(cost.runtime)
            .ok_or(WasmError::WasmGeneratorError("runtime cost".into()))?;
        self.read_count = self
            .read_count
            .checked_sub(cost.read_count)
            .ok_or(WasmError::WasmGeneratorError("read count".into()))?;
        self.read_length = self
            .read_length
            .checked_sub(cost.read_length)
            .ok_or(WasmError::WasmGeneratorError("read length".into()))?;
        self.write_count = self
            .write_count
            .checked_sub(cost.write_count)
            .ok_or(WasmError::WasmGeneratorError("write count".into()))?;
        self.write_length = self
            .write_length
            .checked_sub(cost.write_length)
            .ok_or(WasmError::WasmGeneratorError("write length".into()))?;
        Ok(())
    }
}
