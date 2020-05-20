use crate::assets::vertex::{self, Pos3fTex2f};
use crate::render::{
    Context, Frame, PipelineIndex, PipelineKey, PipelineStore, PipelineStoreRead, TextureIndex, TextureStore,
    TextureStoreRead,
};
use crate::GameError;
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::{resource::Resources, SystemBuilder},
};

const VERTICES: &[Pos3fTex2f] = &[
    Pos3fTex2f {
        position: [-0.0868241, 0.49240386, 0.0],
        texcoord: [0.5, 0.0],
    },
    Pos3fTex2f {
        position: [-0.49513406, 0.06958647, 0.0],
        texcoord: [0.0, 0.5],
    },
    Pos3fTex2f {
        position: [-0.21918549, -0.44939706, 0.0],
        texcoord: [0.0, 0.0],
    },
    Pos3fTex2f {
        position: [0.35966998, -0.3473291, 0.0],
        texcoord: [0.5, 0.5],
    },
    Pos3fTex2f {
        position: [0.44147372, 0.2347359, 0.0],
        texcoord: [0.0, 0.5],
    },
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

struct TestScene {
    pipeline: Option<PipelineIndex>,
    texture: Option<TextureIndex>,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
}

impl TestScene {
    fn new() -> TestScene {
        TestScene {
            pipeline: None,
            texture: None,
            buffers: None,
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
        encoder: &mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'_, '_>,
        pipelines: &mut PipelineStoreRead<'_>,
        textures: &mut TextureStoreRead<'_>,
    ) {
        let pipeline = self.pipeline.get_or_insert_with(|| {
            pipelines.get_or_add_blocking(&PipelineKey::new::<vertex::Pos3fTex2f>(
                "1910/2aa508b774c6f92ec05d1bfb7d53f97eaca1b9c6f9c6082870b1a65b1270.pl",
            ))
        });

        let texture = self.texture.get_or_insert_with(|| {
            textures.get_or_add_blocking(
                &"6832/55ae74cfa024e4cd2333c60aa24a2aceeb1886f5cce102095519ce5ae2df.tex".to_owned(),
            )
        });

        if let Some(ref buffers) = self.buffers {
            let pipeline = pipelines.at(pipeline);
            let _texture = textures.at(texture);
            //let pipeline = &pipelines[pipeline];
            if let Some(mut pipeline) = pipeline.bind(encoder, pass_descriptor) {
                pipeline.set_vertex_buffer(0, buffers.0.slice(..));
                pipeline.set_index_buffer(buffers.1.slice(..));
                pipeline.draw_indexed(0..buffers.2, 0, 0..1);
            }
        }
    }
}

/// Add required resource for the test scene
pub async fn add_test_scene(resources: &mut Resources) -> Result<(), GameError> {
    log::info!("adding test scene to the world");

    resources.insert(TestScene::new());

    Ok(())
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

                scene.render(&mut encoder, &pass_descriptor, &mut pipelines, &mut textures);
            }

            frame.add_command(encoder.finish());
        })
}

pub fn create_schedule() -> Schedule {
    Schedule::builder().add_system(render_test()).flush().build()
}
