use crate::assets::{
    vertex::{self, Pos3fTex2f},
    UniformFormat, UniformSemantic,
};
use crate::render::{
    Context, Frame, PipelineId, PipelineKey, PipelineStore, PipelineStoreRead, TextureId, TextureStore,
    TextureStoreRead,
};
use crate::{GameError, GameRender};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};

const VERTICES: &[Pos3fTex2f] = &[
    Pos3fTex2f {
        position: [-0.0868241, 0.49240386, 0.0],
        texcoord: [0.4131759, 0.00759614],
    },
    Pos3fTex2f {
        position: [-0.49513406, 0.06958647, 0.0],
        texcoord: [0.0048659444, 0.43041354],
    },
    Pos3fTex2f {
        position: [-0.21918549, -0.44939706, 0.0],
        texcoord: [0.28081453, 0.949397057],
    },
    Pos3fTex2f {
        position: [0.35966998, -0.3473291, 0.0],
        texcoord: [0.85967, 0.84732911],
    },
    Pos3fTex2f {
        position: [0.44147372, 0.2347359, 0.0],
        texcoord: [0.9414737, 0.2652641],
    },
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

#[derive(Debug, Serialize, Deserialize)]
pub struct Test3 {
    pub pipeline: String,
    pub texture: String,
}

struct TestScene {
    pipeline: PipelineId,
    texture: TextureId,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    bind_group: Option<wgpu::BindGroup>,
}

impl TestScene {
    fn new(test: Test3) -> TestScene {
        TestScene {
            pipeline: PipelineId::from_key(PipelineKey::new::<vertex::Pos3fTex2f>(&test.pipeline)),
            texture: TextureId::from_key(test.texture),
            buffers: None,
            bind_group: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device) {
        self.buffers.get_or_insert_with(|| {
            (
                device.create_buffer_with_data(bytemuck::cast_slice(VERTICES), wgpu::BufferUsage::VERTEX),
                device.create_buffer_with_data(bytemuck::cast_slice(INDICES), wgpu::BufferUsage::INDEX),
                INDICES.len() as u32,
            )
        });
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'_, '_>,
        pipelines: &mut PipelineStoreRead<'_>,
        textures: &mut TextureStoreRead<'_>,
    ) {
        let pipeline = self.pipeline.get(pipelines);
        let texture = self.texture.get(textures);

        if let Some(ref buffers) = self.buffers {
            if let (Some(pipeline), Some(texture)) = (pipeline.pipeline_buffer(), texture.texture_buffer()) {
                let bind_group = self.bind_group.get_or_insert_with(|| {
                    pipeline
                        .create_global_bind_group(device, |s, f| match (s, f) {
                            (UniformSemantic::Diffuse, UniformFormat::Texture) => {
                                wgpu::BindingResource::TextureView(&texture.view)
                            }
                            (UniformSemantic::Diffuse, UniformFormat::Sampler) => {
                                wgpu::BindingResource::Sampler(&texture.sampler)
                            }
                            _ => unreachable!(),
                        })
                        .unwrap()
                });

                let mut pass = pipeline.bind(encoder, pass_descriptor);
                pass.set_vertex_buffer(0, buffers.0.slice(..));
                pass.set_index_buffer(buffers.1.slice(..));
                pass.set_bind_group(0, bind_group, &[]);
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
        .read_resource::<TextureStore>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, frame, pipelines, textures, scene), _| {
            let mut pipelines = pipelines.read();
            let mut textures = textures.read();

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

                scene.render(
                    context.device(),
                    &mut encoder,
                    &pass_descriptor,
                    &mut pipelines,
                    &mut textures,
                );
            }

            frame.add_command(encoder.finish());
        })
}

pub async fn register_test_scene(test: Test3, game: &mut GameRender) -> Result<(), GameError> {
    log::info!("Adding test3 scene to the world");

    game.resources.insert(TestScene::new(test));

    let render = Schedule::builder().add_system(render_test()).flush().build();
    game.schedules.insert("render", render)?;

    Ok(())
}

pub async fn unregister_test_scene(game: &mut GameRender) -> Result<(), GameError> {
    log::info!("Removing test3 scene from the world");

    game.schedules.remove("render");
    let _ = game.resources.remove::<TestScene>();

    Ok(())
}