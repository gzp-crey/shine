use crate::assets::{AssetIO, Url};
use crate::{Config, GameError};
use shine_ecs::core::store::{Data, DataLoader, Store};
use shine_ecs::legion::systems::resource::Resources;
use std::sync::Arc;

mod surface;
pub use self::surface::*;
mod context;
pub use self::context::*;
mod frame;
pub use self::frame::*;

mod shader;
pub use self::shader::{Shader, ShaderDependency, ShaderIndex, ShaderLoader, ShaderStore, ShaderStoreRead, ShaderType};
mod pipeline;
pub use self::pipeline::{
    Pipeline, PipelineId, PipelineIndex, PipelineKey, PipelineLoader, PipelineStore, PipelineStoreRead,
};
mod model;
pub use self::model::{Model, ModelIndex, ModelLoader, ModelStore, ModelStoreRead};
mod texture;
pub use self::texture::{Texture, TextureId, TextureIndex, TextureLoader, TextureStore, TextureStoreRead};

pub mod tech;

fn register_store<D: Data, L: DataLoader<D>>(loader: L, store_page_size: usize, resources: &mut Resources) {
    let (store, loader) = Store::<D>::new_with_loader(store_page_size, loader);
    resources.insert(store);
    loader.start();
}

/// Add required resource to handle rendering.
/// - *Surface* (thread local) stores for the rendering window surface.
/// - *Context* stores the render surface, driver and queue.
/// - *Frame* stores the current render frame.
/// - *Shader* store
/// - *Pipeline* store
pub async fn add_render_system(
    config: &Config,
    wgpu_instance: wgpu::Instance,
    resources: &mut Resources,
) -> Result<(), GameError> {
    log::info!("adding render system to the world");

    resources.insert(Context::new(wgpu_instance, config).await?);
    resources.insert(Frame::new());

    let base_url = Url::parse(&config.asset_base)
        .map_err(|err| GameError::Config(format!("Failed to parse base url for assets: {:?}", err)))?;
    let assetio =
        Arc::new(AssetIO::new().map_err(|err| GameError::Config(format!("Failed to init assetio: {:?}", err)))?);

    register_store(ShaderLoader::new(assetio.clone(), base_url.clone()), 16, resources);
    register_store(PipelineLoader::new(assetio.clone(), base_url.clone()), 16, resources);
    register_store(ModelLoader::new(assetio.clone(), base_url.clone()), 16, resources);
    register_store(TextureLoader::new(assetio.clone(), base_url.clone()), 16, resources);

    Ok(())
}

pub mod systems {
    pub use super::model::systems::*;
    pub use super::pipeline::systems::*;
    pub use super::shader::systems::*;
    pub use super::texture::systems::*;
}
