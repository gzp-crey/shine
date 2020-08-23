use crate::{input::InputError, render::RenderError};

#[derive(Debug)]
pub enum GameError {
    Config(String),
    Render(RenderError),
    Input(InputError),
}

impl From<RenderError> for GameError {
    fn from(err: RenderError) -> GameError {
        GameError::Render(err)
    }
}

impl From<InputError> for GameError {
    fn from(err: InputError) -> GameError {
        GameError::Input(err)
    }
}
