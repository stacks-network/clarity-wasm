use crate::clarity;

pub mod environments;
pub mod blocks;

pub use self::environments::TestEnv;
pub use blocks::{Block, BlockCursor};

pub enum ImportSource<'a> {
    LocalNode { node_root: &'a str},
    Network { host: &'a str, port: u16, public_key: &'a str }
}

/// Represents a Clarity smart contract.
#[derive(Debug)]
pub struct Contract {
    analysis: clarity::ContractAnalysis,
}

impl Contract {
    pub fn new(analysis: clarity::ContractAnalysis) -> Self {
        Self { analysis }
    }

    pub fn contract_analysis(&self) -> &clarity::ContractAnalysis {
        &self.analysis
    }
}

pub struct TestEnvContext<'a> {
    env: &'a mut TestEnv<'a>
}

impl<'a> TestEnvContext<'a> {
    pub fn new(env: &'a mut TestEnv<'a>) -> Self {
        Self { env }
    }

    pub fn env(&mut self) -> &'a mut TestEnv {
        self.env
    }

    
}


