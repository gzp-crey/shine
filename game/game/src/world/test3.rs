use crate::{
    assets::{
        vertex::{self, Pos3fTex2f},
        TextureSemantic, Uniform, GLOBAL_UNIFORMS,
    },
    render::{
        Context, Frame, PipelineDependency, PipelineStore, PipelineStoreRead, TextureDependency, TextureStore,
        TextureStoreRead, DEFAULT_PASS,
    },
    world::{GameLoadWorld, GameUnloadWorld},
    GameError, GameView,
};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};
use wgpu::util::DeviceExt;

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
        texcoord: [0.28081453, 0.949397],
    },
    Pos3fTex2f {
        position: [0.35966998, -0.3473291, 0.0],
        texcoord: [0.85967, 0.84732914],
    },
    Pos3fTex2f {
        position: [0.44147372, 0.2347359, 0.0],
        texcoord: [0.9414737, 0.2652641],
    },
];

// const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4]; workaround for Buffers that are mapped at creation have to be aligned to COPY_BUFFER_ALIGNMENT'
const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, 0];
const INDEX_COUNT: usize = 9;

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test3 {
    pub pipeline: String,
    pub texture: String,
}
/// Manage the lifecycle of the test
pub struct Test3World;

impl GameLoadWorld for Test3World {
    type Source = Test3;

    fn build(source: Test3, game: &mut GameView) -> Result<Test3World, GameError> {
        log::info!("Adding test3 scene to the world");

        game.resources.insert(TestScene::new(source));

        game.schedules.insert(
            "render",
            Schedule::builder()
                .add_system(prepare_render())
                .flush()
                .add_system(render())
                .flush()
                .build(),
        )?;

        Ok(Test3World)
    }
}

impl GameUnloadWorld for Test3World {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test3 scene from the world");

        game.schedules.remove("render");
        let _ = game.resources.remove::<TestScene>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    pipeline: PipelineDependency,
    texture: TextureDependency,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    bind_group: Option<wgpu::BindGroup>,
}

impl TestScene {
    fn new(test: Test3) -> TestScene {
        TestScene {
            pipeline: PipelineDependency::new()
                .with_id(test.pipeline)
                .with_vertex_layout::<vertex::Pos3fTex2f>(),
            texture: TextureDependency::new().with_id(test.texture),
            buffers: None,
            bind_group: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device) {
        self.buffers.get_or_insert_with(|| {
            log::trace!("creating buffers");
            let v = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            });
            log::trace!("creating buffers2");
            let i = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsage::INDEX,
            });
            log::trace!("creating buffers3");
            (v, i, INDEX_COUNT/*INDICES.len()*/ as u32)
        });
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        pipelines: &PipelineStoreRead<'_>,
        textures: &TextureStoreRead<'_>,
    ) {
        self.pipeline.or_state(frame.get_pipeline_state(DEFAULT_PASS).unwrap());
        
        if let (Some(ref buffers), Ok(Some(pipeline)), Ok(Some(texture))) = (
            &self.buffers,
            self.pipeline.request(pipelines),
            self.texture.request(textures),
        ) {
            let bind_group = self.bind_group.get_or_insert_with(|| {
                pipeline
                    .create_bind_group(GLOBAL_UNIFORMS, device, |u| match u {
                        Uniform::Texture(TextureSemantic::Diffuse) => wgpu::BindingResource::TextureView(&texture.view),
                        Uniform::Sampler(TextureSemantic::Diffuse) => wgpu::BindingResource::Sampler(&texture.sampler),
                        _ => unreachable!(),
                    })
                    .unwrap()
            });

            {
                if let Ok(mut pass) = frame.create_pass(encoder, DEFAULT_PASS) {
                    pass.set_pipeline(&pipeline.pipeline);
                    pass.set_vertex_buffer(0, buffers.0.slice(..));
                    pass.set_index_buffer(buffers.1.slice(..));
                    pass.set_bind_group(GLOBAL_UNIFORMS, &bind_group, &[]);
                    pass.draw_indexed(0..buffers.2, 0, 0..1);
                }
            }
        }
    }
}

fn prepare_render() -> Box<dyn Schedulable> {
    SystemBuilder::new("prepare_render")
        .read_resource::<Context>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, scene), _| {
            scene.prepare(&context.device());
        })
}

fn render() -> Box<dyn Schedulable> {
    SystemBuilder::new("render")
        .read_resource::<Context>()
        .read_resource::<Frame>()
        .read_resource::<PipelineStore>()
        .read_resource::<TextureStore>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, frame, pipelines, textures, scene), _| {
            scene.prepare(&context.device());

            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(
                context.device(),
                &mut encoder,
                &frame,
                &pipelines.read(),
                &textures.read(),
            );

            frame.add_command(encoder.finish());
        })
}
