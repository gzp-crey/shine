use crate::{assets::AssetError, input::InputError, render::RenderError};

#[derive(Debug)]
pub enum GameError {
    Config(String),
    Asset(AssetError),
    Input(InputError),
    Render(RenderError),
}

impl From<AssetError> for GameError {
    fn from(err: AssetError) -> GameError {
        GameError::Asset(err)
    }
}

impl From<InputError> for GameError {
    fn from(err: InputError) -> GameError {
        GameError::Input(err)
    }
}

impl From<RenderError> for GameError {
    fn from(err: RenderError) -> GameError {
        match err {
            RenderError::Asset(err) => GameError::Asset(err),
            err => GameError::Render(err),
        }
    }
}
