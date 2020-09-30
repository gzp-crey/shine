use crate::{app::AppError, assets::AssetConfig, render::RenderConfig};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub asset: AssetConfig,
    pub render: RenderConfig,
}

impl Config {
    fn add_defaults(s: &mut config::Config) -> Result<(), AppError> {
        use config::{File, FileFormat};

        s.merge(File::from_str(
            r#"{
                "render": {
                    "swap_chain_format": "Bgra8UnormSrgb",
                    "enable_validation": false
                }
            }"#,
            FileFormat::Json,
        ))?;
        Ok(())
    }

    pub fn new() -> Result<Self, AppError> {
        use config::{Environment, File};
        let mut s = config::Config::new();

        Self::add_defaults(&mut s)?;

        s.merge(Environment::new().separator("--"))?;

        if let Some(config_file) = env::args().nth(1) {
            log::info!("Loading cofig file {:?}", config_file);
            s.merge(File::from(Path::new(&config_file)))?;
        }

        let cfg = s.try_into()?;

        log::info!("configuration: {:#?}", cfg);
        Ok(cfg)
    }
}

impl FromStr for Config {
    type Err = AppError;

    fn from_str(cfg: &str) -> Result<Self, AppError> {
        use config::{File, FileFormat};
        let mut s = config::Config::new();

        Self::add_defaults(&mut s)?;

        s.merge(File::from_str(cfg, FileFormat::Json))?;

        let cfg = s.try_into()?;

        log::info!("configuration: {:#?}", cfg);
        Ok(cfg)
    }
}
