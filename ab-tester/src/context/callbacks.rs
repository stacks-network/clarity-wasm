use std::path::Path;

use super::Network;
use crate::environments::{ReadableEnv, RuntimeEnv, WriteableEnv};

#[allow(unused_variables)]
pub trait RuntimeEnvCallbackHandler {
    fn get_chain_tip_start(&self, env: &dyn RuntimeEnv) {}
    fn get_chain_tip_finish(&self, env: &dyn RuntimeEnv, tip_height: u32) {}
    fn load_block_headers_start(&self, env: &dyn RuntimeEnv) {}
    fn load_block_headers_iter(&self, env: &dyn RuntimeEnv, iter_height: usize) {}
    fn load_block_headers_finish(&self, env: &dyn RuntimeEnv, header_count: usize) {}

    fn env_open_start(&self, env: &dyn RuntimeEnv, working_dir: &Path) {}
    fn env_open_finish(&self, env: &dyn RuntimeEnv) {}
    fn open_index_db_start(&self, env: &dyn RuntimeEnv, path: &Path) {}
    fn open_index_db_finish(&self, env: &dyn RuntimeEnv) {}
    fn determine_network_start(&self, env: &dyn RuntimeEnv) {}
    fn determine_network_finish(&self, env: &dyn RuntimeEnv, network: &Network) {}
    fn load_db_config_start(&self, env: &dyn RuntimeEnv) {}
    fn load_db_config_finish(&self, env: &dyn RuntimeEnv) {}
    fn open_chainstate_start(&self, env: &dyn RuntimeEnv, path: &Path) {}
    fn open_chainstate_finish(&self, env: &dyn RuntimeEnv) {}
    fn open_clarity_db_start(&self, env: &dyn RuntimeEnv, path: &Path) {}
    fn open_clarity_db_finish(&self, env: &dyn RuntimeEnv) {}
    fn open_sortition_db_start(&self, env: &dyn RuntimeEnv, path: &Path) {}
    fn open_sortition_db_finish(&self, env: &dyn RuntimeEnv) {}
}

#[derive(Clone, Default)]
pub struct DefaultEnvCallbacks {}
impl RuntimeEnvCallbackHandler for DefaultEnvCallbacks {}

#[allow(unused_variables)]
pub trait ReplayCallbackHandler {
    fn replay_start<S: ReadableEnv + ?Sized, T: ReadableEnv + ?Sized>(
        &self,
        source: &S,
        target: &T,
        block_count: usize,
    ) {
    }
    fn replay_finish<S: ReadableEnv + ?Sized, T: ReadableEnv + ?Sized>(
        &self,
        source: &S,
        target: &T,
    ) {
    }
    fn replay_block_start<S: ReadableEnv + ?Sized, T: ReadableEnv + ?Sized>(
        &self,
        source: &S,
        target: &T,
        height: u32,
    ) {
    }
    fn replay_block_finish<S: ReadableEnv + ?Sized, T: ReadableEnv + ?Sized>(
        &self,
        source: &S,
        target: &T,
    ) {
    }
    fn replay_tx_start<E: ReadableEnv + ?Sized>(&self, source: &E, target: &E) {}
    fn replay_tx_finish<E: ReadableEnv + ?Sized>(&self, source: &E, target: &E) {}
}

#[derive(Clone, Default)]
pub struct DefaultReplayCallbacks {}

impl ReplayCallbackHandler for DefaultReplayCallbacks {}
