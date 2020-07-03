use crate::assets::AssetIO;
use crate::input::{self, InputSystem};
use crate::render::{self, Context, RenderSystem, Surface};
use crate::{Config, GameError, ScheduleSet};
use shine_ecs::core::store::{Data, DataLoader, Store};
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
        let assetio = Arc::new(
            AssetIO::new(config.virtual_schemes.clone())
                .map_err(|err| GameError::Config(format!("Failed to init assetio: {:?}", err)))?,
        );
        let context = Context::new(wgpu_instance, &surface, &config).await?;

        let mut view = GameView {
            assetio,
            surface,
            resources: Resources::default(),
            world: World::new(),
            schedules: ScheduleSet::new(),
        };

        view.add_render_system(context)?;
        view.add_input_system()?;

        view.schedules.insert(
            "prepare_update",
            Schedule::builder()
                .add_system(render::systems::update_shaders())
                .add_system(render::systems::update_textures())
                .add_system(render::systems::update_pipelines())
                .add_system(render::systems::update_frame_graphs())
                .add_system(render::systems::update_models())
                .add_system(input::systems::advance_input_states())
                .flush()
                .build(),
        )?;

        view.schedules.insert(
            "gc",
            Schedule::builder()
                .add_system(render::systems::gc_models())
                .add_system(render::systems::gc_frame_graphs())
                .add_system(render::systems::gc_pipelines())
                .add_system(render::systems::gc_textures())
                .add_system(render::systems::gc_shaders())
                .flush()
                .build(),
        )?;

        Ok(view)
    }

    pub fn register_store<D: Data, L: DataLoader<D>>(&mut self, loader: L, store_page_size: usize) {
        let (store, loader) = Store::<D>::new_with_loader(store_page_size, loader);
        self.resources.insert(store);
        loader.start();
    }

    pub fn run_logic(&mut self, logic: &str) {
        let world = &mut self.world;
        let resources = &mut self.resources;
        self.schedules.execute(logic, world, resources);
    }

    pub fn refresh(&mut self, size: (u32, u32)) -> Result<(), GameError> {
        self.run_logic("prepare_update");
        self.run_logic("update");
        //log::trace!("render size: {:?}", size);
        self.render(size)?;
        Ok(())
    }

    pub fn gc(&mut self) {
        self.run_logic("gc");
    }
}
