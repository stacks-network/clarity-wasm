use anyhow::{bail, Result};
use log::*;
use serde_derive::Deserialize;
use std::fs;

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
        let config_filename = &std::env::var("CONFIG_FILE")?;

        let contents = match fs::read_to_string(config_filename) {
            Ok(c) => c,
            Err(err) => bail!("Could not read file `{}`: {}", config_filename, err),
        };

        match toml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(err) => bail!("Unable to load data from `{}`: {}", config_filename, err),
        }
    }
}
