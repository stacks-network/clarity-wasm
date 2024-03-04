#[derive(Default)]
pub struct Cost {
    pub runtime: u64,
    pub read_length: u64,
    pub read_count: u64,
    pub write_length: u64,
    pub write_count: u64,
}

impl Cost {
    pub fn free() -> Self {
        Cost {
            ..Default::default()
        }
    }

    pub fn runtime_const(runtime: u64) -> Self {
        Cost {
            runtime,
            ..Default::default()
        }
    }

    pub fn runtime_linear(n: usize, a: u64, b: u64) -> Self {
        Cost {
            runtime: n as u64 * a + b,
            ..Default::default()
        }
    }
}
