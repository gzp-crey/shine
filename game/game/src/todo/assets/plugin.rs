use crate::assets::{AssetError, AssetIO, Url};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

pub type AssetFuture<'a, R> = Pin<Box<dyn Future<Output = R> + 'a>>;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AssetConfig {
    pub virtual_schemes: HashMap<String, Url>,
}

pub trait AssetPlugin {
    /// initialize asset plugin
    fn add_asset_plugin(&mut self, config: AssetConfig) -> AssetFuture<'_, Result<(), AssetError>>;

    /// Deinitialize asset plugin
    fn remove_asset_plugin(&mut self) -> AssetFuture<'_, Result<(), AssetError>>;
}
