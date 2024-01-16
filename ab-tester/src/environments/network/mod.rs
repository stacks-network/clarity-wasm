use color_eyre::Result;

use super::{RuntimeEnv, EnvConfig};
use crate::context::{Network, Runtime};

/// This environment type is read-only and can be used to sync from the Stacks
/// blockchain using the specified [Network]. This environment will process
/// blocks using the specified [Runtime].
///
/// TODO: Not currently implemented, just a placeholder so that I'm forced to
/// think about the possibility of additional types of environments :)
pub struct NetworkEnv {
    working_dir: String,
    runtime: Runtime,
    network: Network,
}

pub struct NetworkEnvConfig {}

impl EnvConfig for NetworkEnvConfig {
    fn working_dir(&self) -> &std::path::Path {
        todo!()
    }
    fn chainstate_index_db_path(&self) -> &std::path::Path {
        todo!()
    }

    fn is_chainstate_app_indexed(&self) -> bool {
        todo!()
    }

    fn blocks_dir(&self) -> &std::path::Path {
        todo!()
    }

    fn sortition_dir(&self) -> &std::path::Path {
        todo!()
    }

    fn sortition_db_path(&self) -> &std::path::Path {
        todo!()
    }

    fn is_sortition_app_indexed(&self) -> bool {
        todo!()
    }

    fn clarity_db_path(&self) -> &std::path::Path {
        todo!()
    }

    fn is_clarity_db_app_indexed(&self) -> bool {
        todo!()
    }
}

impl RuntimeEnv for NetworkEnv {
    fn name(&self) -> String {
        todo!()
    }

    fn is_readonly(&self) -> bool {
        todo!()
    }

    fn is_open(&self) -> bool {
        todo!()
    }

    fn open(&mut self) -> Result<()> {
        todo!()
    }

    fn id(&self) -> i32 {
        todo!()
    }

    fn cfg(&self) -> &dyn EnvConfig {
        todo!()
    }
}
