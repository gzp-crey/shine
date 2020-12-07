use config::ConfigError;
use shine_ecs::ECSError;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error")]
    Config(#[from] ConfigError),

    #[error("Error in plugin {}", plugin)]
    Plugin { plugin: String, source: Box<dyn StdError> },

    #[error("Plugin not configured {}", plugin)]
    PluginDependency { plugin: String, source: Box<dyn StdError> },

    #[error("Task error")]
    TaskError(#[source] ECSError),
}

impl AppError {
    pub fn plugin<S: ToString, E: 'static + StdError>(plugin: S, error: E) -> AppError {
        AppError::Plugin {
            plugin: plugin.to_string(),
            source: error.into(),
        }
    }

    pub fn plugin_dependency<S: ToString, E: 'static + StdError>(plugin: S, error: E) -> AppError {
        AppError::PluginDependency {
            plugin: plugin.to_string(),
            source: error.into(),
        }
    }
}
