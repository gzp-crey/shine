use crate::assets::vertex;
use crate::render::{Context, Frame, PipelineId, PipelineKey, PipelineStore, PipelineStoreRead};
use crate::{GameError, GameRender};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Test1 {
    pub pipeline: String,
}

struct TestScene {
    pipeline: PipelineId,
}

impl TestScene {
    fn new(test: Test1) -> TestScene {
        TestScene {
            pipeline: PipelineId::from_key(PipelineKey::new::<vertex::Null>(&test.pipeline)),
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
                        attachment: frame.texture_view(),
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color {
                            r: 0.0,
                            g: 0.8,
                            b: 0.0,
                            a: 1.0,
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

pub async fn register_test_scene(test: Test1, game: &mut GameRender) -> Result<(), GameError> {
    log::info!("Adding test1 scene to the world");

    game.resources.insert(TestScene::new(test));

    let render = Schedule::builder().add_system(render_test()).flush().build();
    game.schedules.insert("render", render)?;

    Ok(())
}

pub async fn unregister_test_scene(game: &mut GameRender) -> Result<(), GameError> {
    log::info!("Removing test1 scene from the world");

    game.schedules.remove("render");
    let _ = game.resources.remove::<TestScene>();

    Ok(())
}
