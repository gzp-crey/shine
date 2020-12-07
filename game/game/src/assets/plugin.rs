use crate::{
    app::AppError,
    assets::{AssetIO, Url},
    World,
};
use serde::{Deserialize, Serialize};
use shine_ecs::resources::Resource;
use std::collections::HashMap;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AssetConfig {
    pub virtual_schemes: HashMap<String, Url>,
}

impl World {
    /// Name of the asset plugin
    pub fn asset_plugin_name() -> &'static str {
        "asset"
    }

    /// Request the registered asset io outside of a scheduler.
    pub fn asset_io(&self) -> Result<AssetIO, AppError> {
        let res = self
            .resources
            .get::<AssetIO>()
            .map_err(|err| AppError::plugin_dependency(Self::asset_plugin_name(), err))?;
        Ok(res.clone())
    }

    fn add_asset_resource<T: Resource>(&mut self, resource: T) -> Result<(), AppError> {
        let _ = self
            .resources
            .insert(resource)
            .map_err(|err| AppError::plugin(Self::asset_plugin_name(), err))?;
        Ok(())
    }

    pub async fn add_asset_plugin(&mut self, config: AssetConfig) -> Result<(), AppError> {
        log::info!("Adding asset plugin");
        let asset_io = AssetIO::new(config.virtual_schemes)?;
        self.add_asset_resource(asset_io)?;
        Ok(())
    }

    pub async fn remove_asset_plugin(&mut self) -> Result<(), AppError> {
        self.resources.remove::<AssetIO>();
        Ok(())
    }
}
