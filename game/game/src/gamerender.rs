use crate::input::{self, add_input_system};
use crate::render::{self, add_render_system, Context, ShaderStore, Surface};
use crate::utils::runtime::Runtime;
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
            "render".to_owned(),
            Schedule::builder()
                .add_system(render::systems::update_shaders())
                .flush()
                .build(),
        );

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
    pub runtime: Runtime,
    schedules: ScheduleSet,
}

impl GameRender {
    pub async fn new(config: Config, surface: Surface) -> Result<GameRender, GameError> {
        let mut resources = Resources::default();
        let mut world = World::new();
        let mut runtime = Runtime::new();

        add_input_system(&config, &mut resources, &mut world, &mut runtime).await?;
        add_render_system(&config, &mut resources, &mut world, &mut runtime).await?;

        Ok(GameRender {
            surface,
            resources,
            world,
            runtime,
            schedules: ScheduleSet::new(),
        })
    }

    pub fn init_world() {}

    pub fn run_logic(&mut self, logic: &str) {
        log::trace!("logice: {}", logic);
        let world = &mut self.world;
        let resources = &mut self.resources;
        self.schedules.execute(logic, world, resources);
    }

    pub fn update(&mut self) {
        self.run_logic("update");
    }

    pub fn render(&mut self, size: (u32, u32)) {
        // prepare context
        let surface = &mut self.surface;
        surface.set_size(size);
        self.resources.get_mut::<Context>().map(|mut context| {
            context.init_swap_chain(surface);
        });

        self.run_logic("render");
    }

    pub fn gc_all(&mut self) {
        self.run_logic("gc_all");
    }

    pub fn test(&mut self) {
        self.resources.get_mut::<ShaderStore>().map(|mut store| {
            log::info!("test");
            let mut store = store.write();
            store.named_get_or_add(&"main.vs_spv".to_owned());
        });
    }
}
