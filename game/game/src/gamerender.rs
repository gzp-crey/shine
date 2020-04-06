use crate::input::{self, add_input_system};
use crate::render::add_render_system;
use crate::tasks::TaskEngine;
use crate::wgpu;
use crate::GameError;
use shine_ecs::legion::{
    systems::{resource::Resources, schedule::Schedule},
    thread_resources::ThreadResources,
    world::World,
};

struct Logic {
    update: Schedule,
}

impl Logic {
    fn new() -> Logic {
        let update = Schedule::builder()
            .add_system(input::systems::advance_input_states())
            .flush()
            //.add_thread_local_fn(thread_local_example)
            .build();

        Logic { update }
    }
}

pub struct GameRender {
    pub thread_resources: ThreadResources,
    pub resources: Resources,
    pub world: World,
    pub task_engine: TaskEngine,
    logic: Logic,
}

impl GameRender {
    pub async fn new(surface: wgpu::Surface) -> Result<GameRender, GameError> {
        let mut resources = Resources::default();
        let mut thread_resources = ThreadResources::default();
        let mut world = World::new();
        let mut task_engine = TaskEngine::new();

        add_input_system(&mut thread_resources, &mut resources, &mut world, &mut task_engine).await?;
        add_render_system(
            &mut thread_resources,
            &mut resources,
            &mut world,
            &mut task_engine,
            surface,
        )
        .await?;

        Ok(GameRender {
            thread_resources,
            resources,
            world,
            task_engine,
            logic: Logic::new(),
        })
    }

    pub fn init_world() {}

    pub fn update(&mut self) {
        let world = &mut self.world;
        let resources = &mut self.resources;
        self.logic.update.execute(world, resources);
    }
}
