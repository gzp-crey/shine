use crate::tasks::TaskEngine;
use crate::wgpu;
use crate::GameError;
use shine_ecs::legion::{systems::resource::Resources, thread_resources::ThreadResources, world::World};

mod context;
pub use self::context::*;
//mod shader;
//pub use self::shader::*;
pub mod systems;

/// Add required resource to handle inputs.
/// - *Context* stores the render surface, driver and queue.
/// - *Shader* store TBD
/// - *Pipeline* store TBD
pub async fn add_render_system(
    thread_resources: &mut ThreadResources,
    _resources: &mut Resources,
    _world: &mut World,
    _task_engine: &mut TaskEngine,
    surface: wgpu::Surface,
) -> Result<(), GameError> {
    log::info!("adding render system to the world");
    thread_resources.insert(Context::new(surface).await?);
    Ok(())
}
