use crate::context::{Runtime, Network};

/// This environment type is app-specific and will instrument all Clarity-related
/// operations. This environment can be used for comparisons.
pub struct InstrumentedEnv<'a> {
    working_dir: &'a str,
    runtime: Runtime,
    network: Network
}

impl<'a> InstrumentedEnv<'a> {
    /// Creates a new [InstrumentedEnv]. This method expects the provided
    /// `working_dir` to either be uninitialized or be using the same [Runtime]
    /// and [Network] configuration.
    pub fn new(working_dir: &'a str, runtime: Runtime, network: Network) -> Self {
        Self {
            working_dir,
            runtime,
            network
        }
    }
}