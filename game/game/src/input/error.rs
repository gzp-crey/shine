use crate::{app::AppError, World};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Input error")]
pub struct InputError {}

impl From<InputError> for AppError {
    fn from(err: InputError) -> Self {
        AppError::plugin(World::input_plugin_name(), err)
    }
}
