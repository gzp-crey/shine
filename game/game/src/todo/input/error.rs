use crate::WorldError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("World error")]
    WorldError(#[from] WorldError),
}
