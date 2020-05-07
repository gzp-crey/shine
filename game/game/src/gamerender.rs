use crate::input::{self, add_input_system};
use crate::render::{self, add_render_system, tech, Context, Frame, Surface};
use crate::{Config, GameError};
use shine_ecs::legion::{
    systems::{resource::Resources, schedule::Schedule},
    world::World,
};
use std::collections::HashMap;

struct ScheduleSet {
    logics: HashMap<String, Schedule>,
}

impl ScheduleSet {
    fn new() -> ScheduleSet {
        let mut logics = HashMap::new();

        logics.insert(
            "update".to_owned(),
            Schedule::builder()
                .add_system(input::systems::advance_input_states())
                .flush()
                .build(),
        );

        logics.insert(
            "update_render".to_owned(),
            Schedule::builder()
                .add_system(render::systems::update_shaders())
                .add_system(render::systems::update_pipeline())
                .add_system(render::systems::update_models())
                .flush()
                .build(),
        );

        logics.insert("test1".to_owned(), tech::test1::create_schedule());
        logics.insert("test2".to_owned(), tech::test2::create_schedule());

        ScheduleSet { logics }
    }

    fn execute(&mut self, logic: &str, world: &mut World, resources: &mut Resources) {
        if let Some(schedule) = self.logics.get_mut(logic) {
            schedule.execute(world, resources);
        } else {
            log::warn!("logic [{}] not found", logic);
        }
    }
}

pub struct GameRender {
    pub surface: Surface,
    pub resources: Resources,
    pub world: World,
    schedules: ScheduleSet,
}

impl GameRender {
    pub async fn new(config: Config, wgpu_instance: wgpu::Instance, surface: Surface) -> Result<GameRender, GameError> {
        let mut resources = Resources::default();
        let world = World::new();

        add_input_system(&mut resources).await?;
        add_render_system(&config, wgpu_instance, &mut resources).await?;

        tech::test1::add_test_scene(&mut resources).await?;
        tech::test2::add_test_scene(&mut resources).await?;

        Ok(GameRender {
            surface,
            resources,
            world,
            schedules: ScheduleSet::new(),
        })
    }

    pub fn init_world() {}

    pub fn run_logic(&mut self, logic: &str) {
        let world = &mut self.world;
        let resources = &mut self.resources;
        self.schedules.execute(logic, world, resources);
    }

    pub fn update(&mut self) {
        self.run_logic("update");
    }

    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), String> {
        let surface = &mut self.surface;
        surface.set_size(size);
        let mut context = self.resources.get_mut::<Context>().unwrap();
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.start(context.create_frame(surface)?);
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), String> {
        let context = self.resources.get_mut::<Context>().unwrap();
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.end(context.queue());
        Ok(())
    }

    pub fn render(&mut self, size: (u32, u32)) -> Result<(), String> {
        self.run_logic("update_render");

        self.start_frame(size)?;
        self.run_logic("test2");
        self.end_frame()
    }

    pub fn gc_all(&mut self) {
        self.run_logic("gc_all");
    }
}
