use crate::assets::vertex::{self, Pos3fCol4f};
use crate::render::{Context, Frame, PipelineId, PipelineKey, PipelineStore, PipelineStoreRead};
use crate::{GameError, GameRender};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};

const VERTICES: &[Pos3fCol4f] = &[
    Pos3fCol4f {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.0, 1.0],
    },
    Pos3fCol4f {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.0, 0.5, 0.0, 1.0],
    },
    Pos3fCol4f {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.0, 0.0, 0.5, 1.0],
    },
    Pos3fCol4f {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.5, 0.0, 1.0],
    },
    Pos3fCol4f {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.0, 0.5, 0.5, 1.0],
    },
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

#[derive(Debug, Serialize, Deserialize)]
pub struct Test2 {
    pub pipeline: String,
}

struct TestScene {
    pipeline: PipelineId,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
}

impl TestScene {
    fn new(test: Test2) -> TestScene {
        TestScene {
            pipeline: PipelineId::from_key(PipelineKey::new::<vertex::Pos3fCol4f>(&test.pipeline)),
            buffers: None,
        }
    }

    fn prepare(&mut self, device: &wgpu::Device) {
        self.buffers.get_or_insert_with(|| {
            (
                device.create_buffer_with_data(bytemuck::cast_slice(VERTICES), wgpu::BufferUsage::VERTEX),
                device.create_buffer_with_data(bytemuck::cast_slice(INDICES), wgpu::BufferUsage::INDEX),
                INDICES.len() as u32,
            )
        });
    }

    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'_, '_>,
        pipelines: &mut PipelineStoreRead<'_>,
    ) {
        let pipeline = self.pipeline.get(pipelines);

        if let Some(ref buffers) = self.buffers {
            if let Some(pipeline) = pipeline.pipeline_buffer() {
                let mut pass = pipeline.bind(encoder, pass_descriptor);
                pass.set_vertex_buffer(0, buffers.0.slice(..));
                pass.set_index_buffer(buffers.1.slice(..));
                pass.draw_indexed(0..buffers.2, 0, 0..1);
            }
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

            {
                scene.prepare(&context.device());
            }

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

pub async fn register_test_scene(test: Test2, game: &mut GameRender) -> Result<(), GameError> {
    log::info!("Adding test2 scene to the world");

    game.resources.insert(TestScene::new(test));

    let render = Schedule::builder().add_system(render_test()).flush().build();
    game.schedules.insert("render", render)?;

    Ok(())
}

pub async fn unregister_test_scene(game: &mut GameRender) -> Result<(), GameError> {
    log::info!("Removing test2 scene from the world");

    game.schedules.remove("render");
    let _ = game.resources.remove::<TestScene>();

    Ok(())
}
