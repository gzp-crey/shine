use crate::{
    assets::{AssetIO, Url},
    World, WorldError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AssetConfig {
    pub virtual_schemes: HashMap<String, Url>,
}

impl World {
    /// Name of the asset plugin
    fn asset_plugin_name() -> &'static str {
        "asset"
    }

    /// Request the registered asset io outside of a scheduler.
    pub fn asset_io(&self, plugin: &str) -> Result<AssetIO, WorldError> {
        let res = self
            .resources
            .get::<AssetIO>()
            .map_err(|err| WorldError::MissingDependency {
                plugin: plugin.to_owned(),
                depends_on: Self::asset_plugin_name().to_owned(),
                source: err.into(),
            })?;
        Ok(res.clone())
    }

    pub async fn add_asset_plugin(&mut self, config: AssetConfig) -> Result<(), WorldError> {
        log::info!("Adding asset plugin");
        let assetio = AssetIO::new(config.virtual_schemes).map_err(|err| WorldError::Plugin {
            plugin: Self::asset_plugin_name().to_owned(),
            source: err.into(),
        })?;
        let _ = self.resources.insert(assetio).map_err(|err| WorldError::Plugin {
            plugin: Self::asset_plugin_name().to_owned(),
            source: err.into(),
        })?;
        Ok(())
    }

    pub async fn remove_asset_plugin(&mut self) -> Result<(), WorldError> {
        self.resources.remove::<AssetIO>();
        Ok(())
    }
}
