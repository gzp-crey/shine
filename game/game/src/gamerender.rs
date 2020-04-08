use crate::input::{self, add_input_system};
use crate::render::{self, add_render_system};
use crate::tasks::TaskEngine;
use crate::wgpu;
use crate::GameError;
use shine_ecs::legion::{
    systems::{resource::Resources, schedule::Schedule},
    thread_resources::{ThreadResources, WrapThreadResource},
    world::World,
};
use std::collections::HashMap;

struct ScheduleSet {
    wrap_thread_local: WrapThreadResource,
    logics: HashMap<String, Schedule>,
}

impl ScheduleSet {
    fn new() -> ScheduleSet {
        let wrap_thread_local = WrapThreadResource::new();
        let mut logics = HashMap::new();

        logics.insert(
            "update".to_owned(),
            Schedule::builder()
                .add_system(input::systems::advance_input_states())
                .flush()
                .add_thread_local(render::systems::prepare_context(wrap_thread_local.clone()))
                .build(),
        );

        ScheduleSet {
            wrap_thread_local,
            logics,
        }
    }

    fn execute(
        &mut self,
        logic: &str,
        world: &mut World,
        resources: &mut Resources,
        thread_resources: &mut ThreadResources,
    ) {
        if let Some(schedule) = self.logics.get_mut(logic) {
            self.wrap_thread_local.wrap(thread_resources);
            schedule.execute(world, resources);
            self.wrap_thread_local.unwrap();
        }
    }
}

pub struct GameRender {
    pub thread_resources: ThreadResources,
    pub resources: Resources,
    pub world: World,
    pub task_engine: TaskEngine,
    schedules: ScheduleSet,
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
            schedules: ScheduleSet::new(),
        })
    }

    pub fn init_world() {}

    pub fn run_logic(&mut self, logic: &str) {
        log::trace!("logice: {}", logic);
        let world = &mut self.world;
        let resources = &mut self.resources;
        let thread_resources = &mut self.thread_resources;
        self.schedules.execute("update", world, resources, thread_resources);
    }

    pub fn update(&mut self) {
        self.run_logic("update");
    }

    pub fn render(&mut self, _size: (u32, u32)) {
        //todo: get context, set requetsed size
        self.run_logic("render");
    }
}
