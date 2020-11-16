use crate::{assets::AssetError, input::InputError, render::RenderError, WorldError};
use shine_ecs::ecs::ECSError;
use config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error")]
    Config(#[from] ConfigError),

    #[error("Entity system error")]
    ECSError(#[from] ECSError),

    #[error(transparent)]
    World(#[from] WorldError),

    #[error(transparent)]
    Asset(#[from] AssetError),

    #[error(transparent)]
    Render(#[from] RenderError),

    #[error(transparent)]
    Input(#[from] InputError),
}
