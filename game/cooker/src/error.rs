use shine_game::assets::{AssetError, UrlError};
use std::{error, fmt};
use tokio::task;

#[derive(Debug)]
pub enum CookingError {
    Asset(AssetError),
    Runtime(task::JoinError),
    Json(serde_json::Error),
    Bincode(bincode::Error),
    Db(String),
    Other(String),
}

impl fmt::Display for CookingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CookingError::Asset(ref err) => write!(f, "Asset error: {}", err),
            CookingError::Runtime(ref err) => write!(f, "Runtime error: {}", err),
            CookingError::Json(ref err) => write!(f, "Json error: {}", err),
            CookingError::Bincode(ref err) => write!(f, "Binary serialize error: {}", err),
            CookingError::Db(ref err) => write!(f, "Db error: {}", err),
            CookingError::Other(ref err) => write!(f, "Cooking failed: {}", err),
        }
    }
}

impl error::Error for CookingError {}

impl From<UrlError> for CookingError {
    fn from(err: UrlError) -> CookingError {
        AssetError::from(err).into()
    }
}

impl From<AssetError> for CookingError {
    fn from(err: AssetError) -> CookingError {
        CookingError::Asset(err)
    }
}

impl From<serde_json::Error> for CookingError {
    fn from(err: serde_json::Error) -> CookingError {
        CookingError::Json(err)
    }
}

impl From<bincode::Error> for CookingError {
    fn from(err: bincode::Error) -> CookingError {
        CookingError::Bincode(err)
    }
}

impl From<task::JoinError> for CookingError {
    fn from(err: task::JoinError) -> CookingError {
        CookingError::Runtime(err)
    }
}

impl From<sqlx::Error> for CookingError {
    fn from(err: sqlx::Error) -> CookingError {
        CookingError::Db(format!("{}", err))
    }
}
