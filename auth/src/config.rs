use crate::auth::AuthConfig;
use config::{self, ConfigError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub worker_count: usize,
    pub cookie_user_id_secret: String,
    pub auth: AuthConfig,
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
                "worker_count": "4",
                "cookie_user_id_secret" : "ERROR: provide secret from secret.config.json",
                "auth" : {
                    "identity": {                        
                        "password_pepper": "ERROR: provide secret from secret.config.json",
                        "user_id_secret": "ERROR: provide secret from secret.config.json",
                        "login_key_secret": "ERROR: provide secret from secret.config.json",
                        "storage_account": "ERROR: provide secret from secret.config.json",
                        "storage_account_key": "ERROR: provide secret from secret.config.json"
                    }
                }
            }
            "#,
            FileFormat::Json,
        ))?;

        s.merge(Environment::new().separator("--"))?;

        log::info!("The current directory is {:?}", std::env::current_dir());

        match s.merge(File::from(Path::new("secret.config.json"))) {
            Ok(_) => {}
            Err(err) => log::warn!("Faild to parse secret config: {}", err),
        };

        s.try_into()
    }

    pub fn get_bind_address(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }
}
