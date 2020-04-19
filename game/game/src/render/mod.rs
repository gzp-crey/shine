use crate::utils::runtime::{Runtime, TaskSpawner};
use crate::{Config, GameError};
use shine_ecs::legion::{systems::resource::Resources, world::World};

mod surface;
pub use self::surface::*;
mod context;
pub use self::context::*;
mod shader;
pub use self::shader::*;
pub mod systems;

/// Add required resource to handle rendering.
/// - *Surface* (thread local) stores for the rendering window surface.
/// - *Context* stores the render surface, driver and queue.
/// - *Shader* store
/// - *Pipeline* store TBD
pub async fn add_render_system(
    config: &Config,
    resources: &mut Resources,
    _world: &mut World,
    runtime: &mut Runtime,
) -> Result<(), GameError> {
    log::info!("adding render system to the world");

    resources.insert(Context::new().await?);

    let shader_loader = ShaderLoader::new(&config.asset_base)?;
    let (shader_store, shader_loader) = ShaderStore::new_with_loader(16, shader_loader);
    resources.insert(shader_store);
    runtime.spawn_background_task(shader_loader.run());

    Ok(())
}
