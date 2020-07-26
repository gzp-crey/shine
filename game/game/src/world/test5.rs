use crate::assets::vertex;
use crate::render::{Context, Frame, PipelineKey, PipelineNamedId, PipelineStore, PipelineStoreRead};
use crate::world::{GameLoadWorld, GameUnloadWorld};
use crate::{GameError, GameView};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};
use std::borrow::Cow;

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test5 {
    pub frame_graph: String,
    pub pipeline: String,
}

/// Manage the lifecycle of the test
pub struct Test5World;

impl GameLoadWorld for Test5World {
    type Source = Test5;

    fn build(source: Test5, game: &mut GameView) -> Result<Test5World, GameError> {
        log::info!("Adding test5 scene to the world");

        game.resources.insert(TestScene::new(source));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(Test5World)
    }
}

impl GameUnloadWorld for Test5World {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test5 scene from the world");

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
    fn new(test: Test5) -> TestScene {
        TestScene {
            pipeline: PipelineNamedId::from_key(PipelineKey::new::<vertex::Null>(&test.pipeline)),
        }
    }

    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &Frame, pipelines: &mut PipelineStoreRead<'_>) {
        if let Some(pipeline) = self.pipeline.get(pipelines).pipeline_buffer() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: Cow::Borrowed(&[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: true,
                    },
                }]),
                depth_stencil_attachment: None,
            });

            pass.set_pipeline(&pipeline.pipeline);
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
            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(&mut encoder, &frame, &mut pipelines.read());
            frame.add_command(encoder.finish());
        })
}
