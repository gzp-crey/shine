use crate::{
    assets::{AssetError, AssetIO, Url},
    World,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin};

pub type AssetFuture<'a, R> = Pin<Box<dyn Future<Output = R> + 'a>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetConfig {
    pub virtual_schemes: HashMap<String, Url>,
}

pub trait AssetPlugin {
    /// Add asset handler plugin to the world
    fn add_asset_plugin(&mut self, config: AssetConfig) -> AssetFuture<'_, Result<(), AssetError>>;

    /// Remove asset handler plugin from the world
    fn remove_asset_plugin(&mut self) -> AssetFuture<'_, Result<(), AssetError>>;
}

impl AssetPlugin for World {
    fn add_asset_plugin(&mut self, config: AssetConfig) -> AssetFuture<'_, Result<(), AssetError>> {
        Box::pin(async move {
            log::info!("Adding asset plugin");
            let assetio = AssetIO::new(config.virtual_schemes)?;
            self.resources.insert(assetio);
            Ok(())
        })
    }

    fn remove_asset_plugin(&mut self) -> AssetFuture<'_, Result<(), AssetError>> {
        Box::pin(async move {
            let _ = self.resources.remove::<AssetIO>();
            Ok(())
        })
    }
}