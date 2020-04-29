#[derive(Debug)]
pub enum AssetError {
    AssetNotFound,
    UnsupportedScheme(String),
    ContentLoad(String),
    ContentSave(String),
    TODO,
}

#[cfg(feature = "native")]
mod tokio_assets;
#[cfg(feature = "native")]
pub use self::tokio_assets::*;

#[cfg(feature = "wasm")]
mod wasm_assets;
#[cfg(feature = "wasm")]
pub use self::wasm_assets::*;
