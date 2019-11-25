use crate::auth::AuthConfig;
use config::{self, ConfigError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub worker_count: usize,
    pub user_id_secret: String,
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
                "user_id_secret" : "c2VyZmV0",
                "auth" : {
                    "identity": {
                        "password_pepper": "012",
                        "storage_account": "120",
                        "storage_account_key": "c2VyZmV0"
                    }
                }
            }
            "#,
            FileFormat::Json,
        ))?;

        s.merge(Environment::new().separator("--"))?;

        s.merge(File::from(Path::new("../../secret.config.json")))?;

        s.try_into()
    }

    pub fn get_bind_address(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }
}
