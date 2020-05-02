use crate::{Config, GameError};
use shine_ecs::core::store::{Data, DataLoader, Store};
use shine_ecs::legion::systems::resource::Resources;

mod surface;
pub use self::surface::*;
mod context;
pub use self::context::*;
mod frame;
pub use self::frame::*;
mod shader;
pub use self::shader::{Shader, ShaderDependency, ShaderIndex, ShaderLoader, ShaderStore, ShaderStoreRead, ShaderType};
mod pipeline;
pub use self::pipeline::{Pipeline, PipelineIndex, PipelineKey, PipelineLoader, PipelineStore, PipelineStoreRead};
pub mod pipeline_descriptor;
pub use self::pipeline_descriptor::*;
mod vertex_layout;
pub use self::vertex_layout::*;

pub mod test_tech;

pub mod systems;

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

    register_store(ShaderLoader::new(&config.asset_base)?, 16, resources);
    register_store(PipelineLoader::new(&config.asset_base)?, 16, resources);

    Ok(())
}
