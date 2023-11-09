use super::Network;

#[derive(Clone, Default)]
pub struct RuntimeEnvCallbacks<'a> {
    get_chain_tip_start: Option<&'a dyn Fn()>,
    get_chain_tip_finish: Option<&'a dyn Fn(i32)>,
    load_block_headers_start: Option<&'a dyn Fn()>,
    load_block_headers_iter: Option<&'a dyn Fn(usize)>,
    load_block_headers_finish: Option<&'a dyn Fn()>,

    env_open_start: Option<&'a dyn Fn(&str)>,
    env_open_finish: Option<&'a dyn Fn()>,
    open_index_db_start: Option<&'a dyn Fn(&str)>,
    open_index_db_finish: Option<&'a dyn Fn()>,
    determine_network_start: Option<&'a dyn Fn()>,
    determine_network_finish: Option<&'a dyn Fn(&Network)>,
    load_db_config_start: Option<&'a dyn Fn()>,
    load_db_config_finish: Option<&'a dyn Fn()>,
    open_chainstate_start: Option<&'a dyn Fn(&str)>,
    open_chainstate_finish: Option<&'a dyn Fn()>,
    open_clarity_db_start: Option<&'a dyn Fn(&str)>,
    open_clarity_db_finish: Option<&'a dyn Fn()>,
    open_sortition_db_start: Option<&'a dyn Fn(&str)>,
    open_sortition_db_finish: Option<&'a dyn Fn()>,
}

impl<'a> RuntimeEnvCallbacks<'a> {
    pub fn get_chain_tip_start(&self) {
        if let Some(func) = self.get_chain_tip_start {
            func();
        }
    }

    pub fn get_chain_tip_finish(&self, tip_height: i32) {
        if let Some(func) = self.get_chain_tip_finish {
            func(tip_height);
        }
    }

    pub fn load_block_headers_start(&self) {
        if let Some(func) = self.load_block_headers_start {
            func();
        }
    }

    pub fn load_block_headers_iter(&self, count: usize) {
        if let Some(func) = self.load_block_headers_iter {
            func(count);
        }
    }

    pub fn load_block_headers_finish(&self) {
        if let Some(func) = self.load_block_headers_finish {
            func();
        }
    }

    pub fn env_open_start(&self, name: &str) {
        if let Some(func) = self.env_open_start {
            func(name);
        }
    }

    pub fn env_open_finish(&self) {
        if let Some(func) = self.env_open_finish {
            func();
        }
    }

    pub fn open_index_db_start(&self, path: &str) {
        if let Some(func) = self.open_index_db_start {
            func(path);
        }
    }

    pub fn open_index_db_finish(&self) {
        if let Some(func) = self.open_index_db_finish {
            func();
        }
    }

    pub fn load_db_config_start(&self) {
        if let Some(func) = self.load_db_config_start {
            func();
        }
    }

    pub fn load_db_config_finish(&self) {
        if let Some(func) = self.load_db_config_finish {
            func();
        }
    }

    pub fn determine_network_start(&self) {
        if let Some(func) = self.determine_network_start {
            func();
        }
    }

    pub fn determine_network_finish(&self, network: &Network) {
        if let Some(func) = self.determine_network_finish {
            func(network);
        }
    }

    pub fn open_chainstate_start(&self, path: &str) {
        if let Some(func) = self.open_chainstate_start {
            func(path);
        }
    }

    pub fn open_chainstate_finish(&self) {
        if let Some(func) = self.open_chainstate_finish {
            func();
        }
    }

    pub fn open_clarity_db_start(&self, path: &str) {
        if let Some(func) = self.open_clarity_db_start {
            func(path);
        }
    }

    pub fn open_clarity_db_finish(&self) {
        if let Some(func) = self.open_clarity_db_finish {
            func();
        }
    }

    pub fn open_sortition_db_start(&self, path: &str) {
        if let Some(func) = self.open_sortition_db_start {
            func(path);
        }
    }

    pub fn open_sortition_db_finish(&self) {
        if let Some(func) = self.open_sortition_db_finish {
            func();
        }
    }
}