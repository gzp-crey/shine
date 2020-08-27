use crate::{
    assets::{vertex, FrameGraphDescriptor},
    render::{Context, Frame, PipelineBindGroup, PipelineDependency, RenderPlugin, RenderResources},
    world::{GameLoadWorld, GameUnloadWorld},
    GameError, GameView,
};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test1 {
    pub pipeline: String,
}

/// Manage the lifecycle of the test
pub struct Test1World;

impl GameLoadWorld for Test1World {
    type Source = Test1;

    fn build(source: Test1, game: &mut GameView) -> Result<Test1World, GameError> {
        log::info!("Adding test1 scene to the world");

        game.set_frame_graph(FrameGraphDescriptor::single_pass())?;
        game.resources.insert(TestScene::new(source));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(Test1World)
    }
}

impl GameUnloadWorld for Test1World {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test1 scene from the world");

        game.schedules.remove("render");
        let _ = game.resources.remove::<TestScene>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    pipeline: PipelineDependency,
    bind_group: Option<PipelineBindGroup>,
}

impl TestScene {
    fn new(test: Test1) -> TestScene {
        TestScene {
            pipeline: PipelineDependency::new()
                .with_id(test.pipeline)
                .with_vertex_layout::<vertex::Null>(),
            bind_group: None,
        }
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        resources: &RenderResources,
    ) {
        let pipelines = resources.pipelines.read();
        match frame.begin_pass(encoder, FrameGraphDescriptor::SINGLE_PASS_NAME) {
            Ok(mut pass) => {
                self.pipeline.or_state(pass.get_pipeline_state());
                if let Ok(Some(pipeline)) = self.pipeline.request(&pipelines) {
                    if self.bind_group.is_none() {
                        self.bind_group = Some(pipeline.create_bind_groups(
                            device,
                            |_| unreachable!(),
                            |_| unreachable!(),
                            |_| unreachable!(),
                        ));
                    }

                    pass.set_pipeline(&pipeline, self.bind_group.as_ref().unwrap());
                    pass.draw(0..3, 0..1);
                }
            }
            Err(err) => {
                log::error!("render error: {:?}", err);
            }
        }
    }
}

fn render_test() -> Box<dyn Schedulable> {
    SystemBuilder::new("test_render")
        .read_resource::<Context>()
        .read_resource::<Frame>()
        .read_resource::<RenderResources>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, frame, resources, scene), _| {
            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(context.device(), &mut encoder, &frame, &resources);

            frame.add_command(encoder.finish());
        })
}
