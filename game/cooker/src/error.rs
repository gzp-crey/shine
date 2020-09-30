use shine_game::assets::{AssetError, UrlError};
use thiserror::Error;
use tokio::task;

#[derive(Debug, Error)]
pub enum CookingError {
    #[error("Asset error")]
    Asset(#[from] AssetError),
    #[error("Runtime error")]
    Runtime(#[from] task::JoinError),
    #[error("Serialization error - json")]
    Json(#[from] serde_json::Error),
    #[error("Serialization error - binary")]
    Bincode(#[from] bincode::Error),
    #[error("Database error")]
    SqlDb(#[from] sqlx::Error),
    //Other(String),
}

impl From<UrlError> for CookingError {
    fn from(err: UrlError) -> CookingError {
        AssetError::from(err).into()
    }
}
