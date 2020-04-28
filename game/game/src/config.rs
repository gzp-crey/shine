use crate::GameError;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub asset_base: String,
}

impl Config {
    pub fn new() -> Result<Self, GameError> {
        use config::{Environment, File};
        let mut s = config::Config::new();

        s.merge(Environment::new().separator("--"))
            .map_err(|err| GameError::Config(format!("configuration error in environments: {:?}", err)))?;

        if let Some(config_file) = env::args().skip(1).next() {
            log::info!("Loading cofig file {:?}", config_file);
            s.merge(File::from(Path::new(&config_file))).map_err(|err| {
                GameError::Config(format!("configuration error in file ({}): {:?}", config_file, err))
            })?;
        }

        let cfg = s
            .try_into()
            .map_err(|err| GameError::Config(format!("configuration error: {:?}", err)))?;

        log::info!("configuration: {:#?}", cfg);
        Ok(cfg)
    }

    pub fn from_str(cfg: &str) -> Result<Self, GameError> {
        use config::{File, FileFormat};
        let mut s = config::Config::new();

        s.merge(File::from_str(cfg, FileFormat::Json))
            .map_err(|err| GameError::Config(format!("configuration error in input: {:?}", err)))?;

        let cfg = s
            .try_into()
            .map_err(|err| GameError::Config(format!("configuration error: {:?}", err)))?;

        log::info!("configuration: {:#?}", cfg);
        Ok(cfg)
    }
}
