use crate::{assets::Url, GameError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub virtual_schemes: HashMap<String, Url>,
    pub swap_chain_format: wgpu::TextureFormat,
    pub enable_validation: bool,
    pub wgpu_trace: Option<String>,
}

impl Config {
    fn add_defaults(s: &mut config::Config) -> Result<(), GameError> {
        use config::{File, FileFormat};

        s.merge(File::from_str(
            r#"{
                "swap_chain_format": "Bgra8UnormSrgb",
                "enable_validation": false
            }"#,
            FileFormat::Json,
        ))
        .map_err(|err| GameError::Config(format!("configuration error in defaults: {:?}", err)))?;
        Ok(())
    }

    pub fn new() -> Result<Self, GameError> {
        use config::{Environment, File};
        let mut s = config::Config::new();

        Self::add_defaults(&mut s)?;

        s.merge(Environment::new().separator("--"))
            .map_err(|err| GameError::Config(format!("configuration error in environments: {:?}", err)))?;

        if let Some(config_file) = env::args().nth(1) {
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
}

impl FromStr for Config {
    type Err = GameError;

    fn from_str(cfg: &str) -> Result<Self, GameError> {
        use config::{File, FileFormat};
        let mut s = config::Config::new();

        Self::add_defaults(&mut s)?;

        s.merge(File::from_str(cfg, FileFormat::Json))
            .map_err(|err| GameError::Config(format!("configuration error in input: {:?}", err)))?;

        let cfg = s
            .try_into()
            .map_err(|err| GameError::Config(format!("configuration error: {:?}", err)))?;

        log::info!("configuration: {:#?}", cfg);
        Ok(cfg)
    }
}
