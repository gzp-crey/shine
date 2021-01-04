use config::ConfigError;
use shine_ecs::ECSError;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error")]
    Config(#[from] ConfigError),

    #[error("Plugin already present {}", plugin)]
    PluginAlreadyPresent { plugin: String },

    #[error("Error in plugin {}", plugin)]
    Plugin { plugin: String, source: Box<dyn StdError> },

    #[error("Error in game {}", game)]
    Game { game: String, source: Box<dyn StdError> },

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

    pub fn game<S: ToString, E: 'static + StdError>(game: S, error: E) -> AppError {
        AppError::Game {
            game: game.to_string(),
            source: error.into(),
        }
    }
}
