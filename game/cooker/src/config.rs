use serde::{Deserialize, Serialize};
use shine_game::assets::Url;
use std::collections::HashMap;
use std::env;
use std::path::Path;

use crate::CookerError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub source_root: Url,
    pub source_virtual_schemes: HashMap<String, Url>,

    pub target_db_connection: Option<String>,
    pub target_virtual_schemes: HashMap<String, Url>,
}

impl Config {
    pub fn new() -> Result<Self, CookerError> {
        use config::{Environment, File};
        let mut s = config::Config::new();

        s.merge(Environment::new().separator("--"))?;

        if let Some(config_file) = env::args().skip(1).next() {
            log::info!("Loading cofig file {:?}", config_file);
            s.merge(File::from(Path::new(&config_file)))?;
        }

        let cfg = s.try_into()?;

        log::info!("configuration: {}", serde_json::to_string_pretty(&cfg).unwrap());
        Ok(cfg)
    }
}
