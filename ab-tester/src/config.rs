use anyhow::{bail, Result};
use serde_derive::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub baseline: Baseline,
    pub app: App,
    pub environment: Vec<Environment>,
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
}

#[derive(Debug, Deserialize)]
pub struct Baseline {
    pub chainstate_path: String,
}

#[derive(Debug, Deserialize)]
pub struct App {
    pub db_path: String,
    pub console_theme: Option<String>
}

#[derive(Debug, Deserialize)]
pub struct Environment {
    pub name: String,
    pub runtime: ClarityRuntime,
    pub chainstate_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum ClarityRuntime {
    Interpreter,
    Wasm,
}
