use thiserror::Error;

#[derive(Debug, Error)]
#[error("Input error")]
pub struct InputError {}
