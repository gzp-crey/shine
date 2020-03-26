use crate::input::add_input_system;
use crate::render::Context;
use crate::GameError;
use shine_ecs::world::World;

pub struct GameRender {
    pub context: Context,
    pub world: World,
}

impl GameRender {
    pub async fn new(surface: wgpu::Surface) -> Result<GameRender, GameError> {
        let mut world = World::default();

        add_input_system(&mut world);

        Ok(GameRender {
            context: Context::new(surface).await?,
            world: World::default(),
        })
    }

    pub fn init_world() {}
}
