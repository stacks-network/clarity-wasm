use color_eyre::Result;

use crate::context::{Runtime, Network};

use super::RuntimeEnv;

/// This environment type is read-only and can be used to sync from the Stacks
/// blockchain using the specified [Network]. This environment will process
/// blocks using the specified [Runtime].
/// 
/// TODO: Not currently implemented, just a placeholder so that I'm forced to
/// think about the possibility of additional types of environments :)
pub struct NetworkEnv<'a> {
    working_dir: &'a str,
    runtime: Runtime,
    network: Network
}

impl<'a> RuntimeEnv<'a> for NetworkEnv<'_> {
    fn name(&self) -> &'a str {
        todo!()
    }

    fn is_readonly(&self) -> bool {
        todo!()
    }

    fn network(&self) -> Network {
        todo!()
    }

    fn is_open(&self) -> bool {
        todo!()
    }

    fn open(&mut self) -> Result<()> {
        todo!()
    }
}