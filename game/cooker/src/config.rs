use serde::{Deserialize, Serialize};
use shine_game::assets::Url;
use std::collections::HashMap;
use std::env;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub asset_source_base: Url,
    pub source_virtual_schemes: HashMap<String, Url>,
    pub cache_db_connection: String,

    pub target_db_connection: String,
    pub target_virtual_schemes: HashMap<String, Url>,
}

impl Config {
    pub fn new() -> Result<Self, String> {
        use config::{Environment, File};
        let mut s = config::Config::new();

        s.merge(Environment::new().separator("--"))
            .map_err(|err| format!("configuration error in environments: {:?}", err))?;

        if let Some(config_file) = env::args().skip(1).next() {
            log::info!("Loading cofig file {:?}", config_file);
            s.merge(File::from(Path::new(&config_file)))
                .map_err(|err| format!("configuration error in file ({}): {:?}", config_file, err))?;
        }

        let cfg = s.try_into().map_err(|err| format!("configuration error: {:?}", err))?;

        log::info!("configuration: {:#?}", cfg);
        Ok(cfg)
    }
}
