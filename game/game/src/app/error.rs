use crate::{assets::AssetError, input::InputError, render::RenderError, WorldError};
use config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error")]
    Config(#[from] ConfigError),

    #[error(transparent)]
    World(#[from] WorldError),

    #[error(transparent)]
    Asset(#[from] AssetError),

    #[error(transparent)]
    Render(#[from] RenderError),

    #[error(transparent)]
    Input(#[from] InputError),
}
