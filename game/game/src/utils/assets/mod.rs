use crate::utils::url::UrlError;
use std::{error, fmt};

#[derive(Debug)]
pub enum AssetError {
    UnsupportedScheme(String),
    InvalidUrl(UrlError),
    UnsupportedFormat(String),
    AssetProvider(String),
    ContentLoad(String),
    ContentSave(String),
    Content(String),
    TODO,
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::UnsupportedScheme(scheme) => write!(f, "Unsupported scheme error: {}", scheme),
            AssetError::InvalidUrl(err) => write!(f, "Mallformed url: {}", err),
            AssetError::UnsupportedFormat(ext) => write!(f, "Unsupported input format: {}", ext),
            AssetError::AssetProvider(err) => write!(f, "Asset source error: {}", err),
            AssetError::ContentLoad(err) => write!(f, "Asset loading error: {}", err),
            AssetError::ContentSave(err) => write!(f, "Asset saving error: {}", err),
            AssetError::Content(err) => write!(f, "Error in content: {}", err),
            AssetError::TODO => write!(f, "Not implemented"),
        }
    }
}

impl error::Error for AssetError {}

impl From<UrlError> for AssetError {
    fn from(err: UrlError) -> AssetError {
        AssetError::InvalidUrl(err)
    }
}

#[cfg(feature = "native")]
mod tokio_assets;
#[cfg(feature = "native")]
pub use self::tokio_assets::*;

#[cfg(feature = "wasm")]
mod wasm_assets;
#[cfg(feature = "wasm")]
pub use self::wasm_assets::*;

mod hashing;
pub use hashing::*;
mod utils;
pub use utils::*;
