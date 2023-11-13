use super::{Network, environments::RuntimeEnv};

#[derive(Clone)]
pub struct RuntimeEnvCallbacks<'a> {
    pub get_chain_tip_start: &'a dyn Fn(&dyn RuntimeEnv),
    pub get_chain_tip_finish: &'a dyn Fn(&dyn RuntimeEnv, i32),
    pub load_block_headers_start: &'a dyn Fn(&dyn RuntimeEnv),
    pub load_block_headers_iter: &'a dyn Fn(&dyn RuntimeEnv, usize),
    pub load_block_headers_finish: &'a dyn Fn(&dyn RuntimeEnv),

    pub env_open_start: &'a dyn Fn(&dyn RuntimeEnv, &str),
    pub env_open_finish: &'a dyn Fn(&dyn RuntimeEnv),
    pub open_index_db_start: &'a dyn Fn(&dyn RuntimeEnv, &str),
    pub open_index_db_finish: &'a dyn Fn(&dyn RuntimeEnv),
    pub determine_network_start: &'a dyn Fn(&dyn RuntimeEnv),
    pub determine_network_finish: &'a dyn Fn(&dyn RuntimeEnv, &Network),
    pub load_db_config_start: &'a dyn Fn(&dyn RuntimeEnv),
    pub load_db_config_finish: &'a dyn Fn(&dyn RuntimeEnv),
    pub open_chainstate_start: &'a dyn Fn(&dyn RuntimeEnv, &str),
    pub open_chainstate_finish: &'a dyn Fn(&dyn RuntimeEnv),
    pub open_clarity_db_start: &'a dyn Fn(&dyn RuntimeEnv, &str),
    pub open_clarity_db_finish: &'a dyn Fn(&dyn RuntimeEnv),
    pub open_sortition_db_start: &'a dyn Fn(&dyn RuntimeEnv, &str),
    pub open_sortition_db_finish: &'a dyn Fn(&dyn RuntimeEnv),
}

impl Default for RuntimeEnvCallbacks<'_> {
    fn default() -> Self {

        Self {
            get_chain_tip_start: &|_| {},
            get_chain_tip_finish: &|_, _| {},
            load_block_headers_start: &|_| {},
            load_block_headers_iter: &|_, _| {}, 
            load_block_headers_finish: &|_| {}, 
            env_open_start: &|_, _| {}, 
            env_open_finish: &|_| {}, 
            open_index_db_start: &|_, _| {}, 
            open_index_db_finish: &|_| {}, 
            determine_network_start: &|_| {}, 
            determine_network_finish: &|_, _| {}, 
            load_db_config_start: &|_| {}, 
            load_db_config_finish: &|_| {}, 
            open_chainstate_start: &|_, _| {}, 
            open_chainstate_finish: &|_| {}, 
            open_clarity_db_start: &|_, _| {}, 
            open_clarity_db_finish: &|_| {}, 
            open_sortition_db_start: &|_, _| {}, 
            open_sortition_db_finish: &|_| {} 
        }
    }
}

#[derive(Clone)]
pub struct ReplayCallbacks<'a> {
    pub replay_start: &'a dyn Fn(&dyn RuntimeEnv, &dyn RuntimeEnv, usize),
    pub replay_finish: &'a dyn Fn(&dyn RuntimeEnv, &dyn RuntimeEnv),
    pub replay_block_start: &'a dyn Fn(&dyn RuntimeEnv, &dyn RuntimeEnv, u32, u32),
    pub replay_block_finish: &'a dyn Fn(&dyn RuntimeEnv, &dyn RuntimeEnv),
    pub replay_tx_start: &'a dyn Fn(&dyn RuntimeEnv, &dyn RuntimeEnv),
    pub replay_tx_finish: &'a dyn Fn(&dyn RuntimeEnv, &dyn RuntimeEnv),
}

impl Default for ReplayCallbacks<'_> {
    fn default() -> Self {
        Self { 
            replay_start: &|_, _, _| {}, 
            replay_finish: &|_, _| {}, 
            replay_block_start: &|_, _, _, _| {},
            replay_block_finish: &|_, _| {},
            replay_tx_start: &|_, _| {},
            replay_tx_finish: &|_, _| {}, 
        }
    }
}