use config::{self, ConfigError};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub auth: String,
    pub test_token: String,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        use config::{Environment, File, FileFormat};
        let mut s = config::Config::new();

        s.merge(File::from_str(
            r#"
            {
                "auth": "http://localhost:12345/auth"
            }
            "#,
            FileFormat::Json,
        ))?;

        s.merge(Environment::new().separator("--"))?;

        if let Some(config_file) = env::args().skip(1).next() {
            log::info!("Loading cofig file {:?}", config_file);
            match s.merge(File::from(Path::new(&config_file))) {
                Ok(_) => {}
                Err(err) => log::warn!("Faild to parse config: {}", err),
            };
        }

        s.try_into()
    }
}
