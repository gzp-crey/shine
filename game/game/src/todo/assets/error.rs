use crate::assets::UrlError;
use shine_ecs::core::error::ErrorString;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("Unsupported scheme error: {0}")]
    UnsupportedScheme(String),

    #[error("Mallformed url")]
    InvalidUrl(#[from] UrlError),

    #[error("Unsupported input format: {0}")]
    UnsupportedFormat(String),

    #[error("Asset source error for {content}")]
    ContentSource {
        content: String,
        source: Box<dyn 'static + StdError + Sync + Send>,
    },

    #[error("Asset loading error for {content}")]
    ContentLoad {
        content: String,
        source: Box<dyn 'static + StdError + Sync + Send>,
    },

    #[error("Asset saving error for {content}")]
    ContentSave {
        content: String,
        source: Box<dyn 'static + StdError + Sync + Send>,
    },

    #[error("Error in content: {0}")]
    Content(String),

    #[error("{message}")]
    Other {
        message: String,
        source: Box<dyn 'static + StdError + Sync + Send>,
    },
}

impl AssetError {
    pub fn source_error<S: ToString, E: 'static + StdError + Sync + Send>(content: S, error: E) -> Self {
        AssetError::ContentSource {
            content: content.to_string(),
            source: Box::new(error),
        }
    }

    pub fn source_error_str<S1: ToString, S2: ToString>(content: S1, error: S2) -> Self {
        AssetError::ContentSource {
            content: content.to_string(),
            source: Box::new(ErrorString(error.to_string())),
        }
    }

    pub fn load_failed<S: ToString, E: 'static + StdError + Sync + Send>(content: S, error: E) -> Self {
        AssetError::ContentLoad {
            content: content.to_string(),
            source: Box::new(error),
        }
    }

    pub fn load_failed_str<S1: ToString, S2: ToString>(content: S1, error: S2) -> Self {
        AssetError::ContentLoad {
            content: content.to_string(),
            source: Box::new(ErrorString(error.to_string())),
        }
    }

    pub fn save_failed<S: ToString, E: 'static + StdError + Sync + Send>(content: S, error: E) -> Self {
        AssetError::ContentSave {
            content: content.to_string(),
            source: Box::new(error),
        }
    }

    pub fn save_failed_str<S1: ToString, S2: ToString>(content: S1, error: S2) -> Self {
        AssetError::ContentSave {
            content: content.to_string(),
            source: Box::new(ErrorString(error.to_string())),
        }
    }

    pub fn other<S: ToString, E: 'static + StdError + Sync + Send>(message: S, error: E) -> Self {
        AssetError::Other {
            message: message.to_string(),
            source: Box::new(error),
        }
    }
}

#[cfg(feature = "cook")]
#[derive(Debug, Error)]
pub enum CookingError {
    #[error("Cooking of {:?} failed", content_id)]
    Cook {
        content_id: String,
        source: Box<dyn 'static + StdError + Sync + Send>,
    },
}
