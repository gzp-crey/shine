use crate::{
    app::GameSource,
    app::{AppError, GameFuture, GameLifecycle},
    assets::vertex,
    render::{Context, FrameOutput, PipelineBindGroup, PipelineDependency, RenderResources},
    World,
};
use serde::{Deserialize, Serialize};
use shine_ecs::{
    resources::{Res, ResMut},
    scheduler::{IntoSystemBuilder, Schedule},
};

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test1 {
    pub pipeline: String,
}

impl GameSource for Test1 {
    fn build<'a>(self) -> Result<Box<dyn GameLifecycle>, AppError> {
        Ok(Box::new(self))
    }
}

impl GameLifecycle for Test1 {
    fn create<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            log::info!("Adding test1 scene to the world");

            world.resources.insert(TestScene::new(&self));

            {
                let mut render_schedule = Schedule::default();
                render_schedule.schedule(render.system());
                world.add_stage("render", render_schedule);
            }

            Ok(())
        })
    }

    fn destroy<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            log::info!("Removing test1 scene from the world");

            world.clear_stages();
            let _ = world.resources.remove::<TestScene>();

            Ok(())
        })
    }
}

impl Test1 {
    pub fn render_system(&mut self) -> Schedule {
        let mut schedule = Schedule::default();
        schedule.schedule(render.system());
        schedule
    }
}

/// Resources for the test
struct TestScene {
    pipeline: PipelineDependency,
    bind_group: Option<PipelineBindGroup>,
}

impl TestScene {
    fn new(test: &Test1) -> TestScene {
        TestScene {
            pipeline: PipelineDependency::default()
                .with_id(test.pipeline.clone())
                .with_vertex_layout::<vertex::Null>(),
            bind_group: None,
        }
    }
}

fn render(
    context: Res<Context>,
    frame: Res<FrameOutput>,
    resources: Res<RenderResources>,
    mut scene: ResMut<TestScene>,
) {
    log::error!("render");
    let device = context.device();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    /*let pipelines = resources.pipelines.read();
    match frame.begin_pass(&mut encoder, FrameGraphDescriptor::SINGLE_PASS_NAME) {
        Ok(mut pass) => {
            let scene = &mut *scene;
            scene.pipeline.or_state(pass.get_pipeline_state());
            if let Ok(Some(pipeline)) = scene.pipeline.request(&pipelines) {
                if scene.bind_group.is_none() {
                    log::error!("precre");
                    scene.bind_group = Some(pipeline.create_bind_groups(
                        device,
                        |_| unreachable!(),
                        |_| unreachable!(),
                        |_| unreachable!(),
                    ));
                    log::error!("cre");
                }

                pass.set_pipeline(&pipeline, scene.bind_group.as_ref().unwrap());
                log::error!("predraw");
                pass.draw(0..3, 0..1);
                log::error!("draw");
            }
        }
        Err(err) => {
            log::error!("render error: {:?}", err);
        }
    }*/

    log::error!("pre command");
    context.add_command(encoder.finish());
    log::error!("end command");
}
