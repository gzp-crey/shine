use crate::{
    app::{AppError, Plugin, PluginFuture},
    assets::{AssetIO, Url},
    World,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, error::Error as StdError};

pub const ASSET_PLUGIN_NAME: &str = "asset";

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct AssetConfig {
    pub virtual_schemes: HashMap<String, Url>,
}

pub struct AssetPlugin {
    config: AssetConfig,
}

impl AssetPlugin {
    pub fn new(config: AssetConfig) -> AssetPlugin {
        AssetPlugin { config }
    }
}

fn into_plugin_err<E: 'static + StdError>(error: E) -> AppError {
    AppError::game(ASSET_PLUGIN_NAME, error)
}

impl Plugin for AssetPlugin {
    fn name() -> Cow<'static, str> {
        ASSET_PLUGIN_NAME.into()
    }

    fn init(self, world: &mut World) -> PluginFuture<()> {
        Box::pin(async move {
            let asset_io = AssetIO::new(self.config.virtual_schemes).map_err(into_plugin_err)?;
            world
                .resources
                .register_with_instance(asset_io)
                .map_err(into_plugin_err)?;
            Ok(())
        })
    }

    fn deinit(world: &mut World) -> PluginFuture<()> {
        Box::pin(async move {
            world.resources.unregister::<AssetIO>();
            Ok(())
        })
    }
}
