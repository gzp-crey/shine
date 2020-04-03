use crate::input::add_input_system;
use crate::render::add_render_system;
use crate::GameError;
use shine_ecs::legion::{resource::Resources, thread_resources::ThreadResources, world::World};

pub struct GameRender {
    pub thread_resources: ThreadResources,
    pub resources: Resources,
    pub world: World,
}

impl GameRender {
    pub async fn new(surface: wgpu::Surface) -> Result<GameRender, GameError> {
        let mut resources = Resources::default();
        let mut thread_resources = ThreadResources::default();
        let mut world = World::new();

        add_input_system(&mut thread_resources, &mut resources, &mut world).await?;
        add_render_system(&mut thread_resources, &mut resources, &mut world, surface).await?;

        Ok(GameRender {
            thread_resources,
            resources,
            world,
        })
    }

    pub fn init_world() {}

    pub fn update(&mut self) {
        //world.
    }
}
