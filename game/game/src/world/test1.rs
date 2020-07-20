use crate::assets::vertex;
use crate::render::{Context, Frame, PipelineKey, PipelineNamedId, PipelineStore, PipelineStoreRead};
use crate::world::{GameWorld, GameWorldBuilder};
use crate::{GameError, GameView};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};
use std::any::Any;

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test1 {
    pub pipeline: String,
}

impl GameWorldBuilder for Test1 {
    type World = TestWorld;

    fn build(self, game: &mut GameView) -> Result<TestWorld, GameError> {
        log::info!("Adding test1 scene to the world");

        game.resources.insert(TestScene::new(self));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(TestWorld)
    }
}

/// Manage the lifecycle of the test
pub struct TestWorld;

impl GameWorld for TestWorld {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test1 scene from the world");

        game.schedules.remove("render");
        let _ = game.resources.remove::<TestScene>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    pipeline: PipelineNamedId,
}

impl TestScene {
    fn new(test: Test1) -> TestScene {
        TestScene {
            pipeline: PipelineNamedId::from_key(PipelineKey::new::<vertex::Null>(&test.pipeline)),
        }
    }

    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'_, '_>,
        pipelines: &mut PipelineStoreRead<'_>,
    ) {
        let pipeline = self.pipeline.get(pipelines);

        if let Some(pipeline) = pipeline.pipeline_buffer() {
            let mut pass = pipeline.bind(encoder, pass_descriptor);
            pass.draw(0..3, 0..1);
        }
    }
}

fn render_test() -> Box<dyn Schedulable> {
    SystemBuilder::new("test_render")
        .read_resource::<Context>()
        .read_resource::<Frame>()
        .read_resource::<PipelineStore>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, frame, pipelines, scene), _| {
            let mut pipelines = pipelines.read();

            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            {
                let pass_descriptor = wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output().frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                };

                //log::info!("render pass");
                //let mut render_pass = encoder.begin_render_pass(&pass_descriptor);
                scene.render(&mut encoder, &pass_descriptor, &mut pipelines);
            }

            frame.add_command(encoder.finish());
        })
}
