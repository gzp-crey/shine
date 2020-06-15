use crate::assets::AssetIO;
use crate::input::{self, InputSystem};
use crate::render::{self, Context, Frame, Surface};
use crate::{Config, GameError, ScheduleSet};
use shine_ecs::legion::{
    systems::{resource::Resources, schedule::Schedule},
    world::World,
};
use std::sync::Arc;

pub struct GameView {
    pub assetio: Arc<AssetIO>,
    pub surface: Surface,
    pub resources: Resources,
    pub world: World,
    pub schedules: ScheduleSet,
}

impl GameView {
    pub async fn new(config: Config, wgpu_instance: wgpu::Instance, surface: Surface) -> Result<GameView, GameError> {
        let mut resources = Resources::default();
        let world = World::new();
        let assetio = Arc::new(
            AssetIO::new(config.virtual_schemes.clone())
                .map_err(|err| GameError::Config(format!("Failed to init assetio: {:?}", err)))?,
        );

        render::add_render_system(&config, assetio.clone(), wgpu_instance, &surface, &mut resources).await?;

        let schedules = {
            let mut schedules = ScheduleSet::new();

            schedules.insert(
                "prepare_update",
                Schedule::builder()
                    .add_system(render::systems::update_shaders())
                    .add_system(render::systems::update_textures())
                    .add_system(render::systems::update_pipelines())
                    .add_system(render::systems::update_models())
                    .add_system(input::systems::advance_input_states())
                    .flush()
                    .build(),
            )?;

            schedules.insert(
                "gc_stores",
                Schedule::builder()
                    .add_system(render::systems::gc_models())
                    .add_system(render::systems::gc_pipelines())
                    .add_system(render::systems::gc_textures())
                    .add_system(render::systems::gc_shaders())
                    .flush()
                    .build(),
            )?;

            schedules
        };

        let mut view = GameView {
            assetio,
            surface,
            resources,
            world,
            schedules,
        };

        view.add_input_system()?;
        Ok(view)
    }

    pub fn init_world() {}

    pub fn run_logic(&mut self, logic: &str) {
        let world = &mut self.world;
        let resources = &mut self.resources;
        self.schedules.execute(logic, world, resources);
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
        self.run_logic("prepare_update");
        self.run_logic("update");

        self.start_frame(size)?;
        self.run_logic("render");
        self.end_frame()
    }

    pub fn gc_all(&mut self) {
        self.run_logic("gc_stores");
    }
}
