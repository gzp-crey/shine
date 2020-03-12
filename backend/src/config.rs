use config::{self, ConfigError};
use serde::{Deserialize, Serialize};
use shine_auth::AuthConfig;
use shine_web::WebConfig;
use std::env;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub worker_count: usize,
    pub auth: AuthConfig,
    pub web: WebConfig,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        use config::{Environment, File, FileFormat};
        let mut s = config::Config::new();

        s.merge(File::from_str(
            r#"
            {
                "bind_host": "0.0.0.0",
                "bind_port": "12345",
                "worker_count": "4"
            }
            "#,
            FileFormat::Json,
        ))?;

        s.merge(Environment::new().separator("--"))?;

        if let Some(config_file) = env::args().skip(1).next() {
            log::info!("Loading cofig file {:?}", config_file);
            match s.merge(File::from(Path::new(&config_file))) {
                Ok(_) => {}
                Err(err) => log::warn!("Faild to parse secret config: {}", err),
            };
        }

        s.try_into()
    }

    pub fn get_bind_address(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }
}
