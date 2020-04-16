use crate::tasks::{TaskEngine, TaskSpawner};
use crate::GameError;
use shine_ecs::legion::{systems::resource::Resources, thread_resources::ThreadResources, world::World};

mod context;
pub use self::context::*;
mod shader;
pub use self::shader::*;
pub mod systems;

/// Add required resource to handle inputs.
/// - *Context* stores the render surface, driver and queue.
/// - *Shader* store TBD
/// - *Pipeline* store TBD
pub async fn add_render_system(
    thread_resources: &mut ThreadResources,
    resources: &mut Resources,
    _world: &mut World,
    task_engine: &mut TaskEngine,
) -> Result<(), GameError> {
    log::info!("adding render system to the world");

    resources.insert(Context::new().await?);

    let (shader_store, shader_loader) = ShaderStore::new_with_loader(16, ShaderLoader);
    resources.insert(shader_store);
    task_engine.spawn_background_task(shader_loader.run());

    Ok(())
}
