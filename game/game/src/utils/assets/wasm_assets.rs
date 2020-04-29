#![cfg(feature = "wasm")]

use crate::utils::assets::AssetError;
use crate::utils::url::Url;

pub async fn download_string(url: &Url) -> Result<String, AssetError> {
    unimplemented!()
}

pub async fn download_binary(url: &Url) -> Result<Vec<u8>, AssetError> {
    unimplemented!()
}

pub async fn upload_binary(url: &Url, data: &[u8]) -> Result<(), AssetError> {
    unimplemented!()
}

pub async fn upload_string(url: &Url, data: &str) -> Result<(), AssetError> {
    unimplemented!()
}
