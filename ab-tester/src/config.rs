use anyhow::{bail, Result};
use serde_derive::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Chainstate {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub chainstate: Chainstate,
}

impl Config {
    pub fn load() -> Result<Config> {
        let config_filename = "config.toml";
        let contents = match fs::read_to_string("config.toml") {
            Ok(c) => c,
            Err(err) => bail!("Could not read file `{}`: {}", config_filename, err),
        };

        match toml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(err) => bail!("Unable to load data from `{}`: {}", config_filename, err),
        }
    }

    pub fn get_index_db_path(&self) -> Result<String> {
        let path = Path::new(&self.chainstate.path)
            .join("vm/index.sqlite")
            .to_string_lossy()
            .to_string();

        Ok(path)
    }

    pub fn get_clarity_db_path(&self) -> Result<String> {
        let path = Path::new(&self.chainstate.path)
            .join("vm/clarity/")
            .to_string_lossy()
            .to_string();

        Ok(path)
    }

    pub fn get_blocks_dir(&self) -> Result<String> {
        let path = Path::new(&self.chainstate.path)
            .join("blocks/")
            .to_string_lossy()
            .to_string();

        Ok(path)
    }
}
