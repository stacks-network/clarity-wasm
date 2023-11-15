use std::fs;

use color_eyre::eyre::{bail, Result};
use serde_derive::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub baseline: Baseline,
    pub app: App,
    environment: Vec<Environment>,
}

impl Config {
    pub fn load(path: &str) -> Result<Config> {
        let contents = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(err) => bail!("Could not read file `{}`: {}", path, err),
        };

        match toml::from_str(&contents) {
            Ok(config) => Ok(config),
            Err(err) => bail!("Unable to load data from `{}`: {}", path, err),
        }
    }

    pub fn environments(&self) -> &[Environment] {
        &self.environment
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Baseline {
    pub chainstate_path: String,
    pub chain_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct App {
    pub db_path: String,
    pub console_theme: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Environment {
    pub name: String,
    pub runtime: ClarityRuntime,
    pub chainstate_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ClarityRuntime {
    Interpreter,
    Wasm,
}
