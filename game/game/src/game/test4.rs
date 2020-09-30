use crate::{
    assets::{
        uniform::ViewProj,
        vertex::{self, Pos3fTex2f},
        FrameGraphDescriptor, TextureSemantic, Uniform, UniformSemantic,
    },
    components::camera::{Camera, FirstPerson, Projection},
    input::{mapper::FirstPersonShooter, CurrentInputState, InputMapper, InputPlugin},
    render::{Context, Frame, PipelineBindGroup, PipelineDependency, RenderPlugin, RenderResources, TextureDependency},
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
pub struct Test4 {
    pub pipeline: String,
    pub texture: String,
}

/// Manage the lifecycle of the test
pub struct Test4World;

impl GameLoadWorld for Test4World {
    type Source = Test4;

    fn build(source: Test4, game: &mut GameView) -> Result<Test4World, GameError> {
        log::info!("Adding test4 scene to the world");

        game.set_frame_graph(FrameGraphDescriptor::single_pass())?;
        game.set_input(FirstPersonShooter::new())?;
        game.resources.insert(TestScene::new(source));
        game.resources.insert(FirstPerson::new());
        game.resources.insert(Projection::new());

        game.schedules.insert(
            "update",
            Schedule::builder()
                .add_system(update_camera())
                .add_system(bake_camera::<FirstPerson>())
                .flush()
                .build(),
        )?;

        game.schedules.insert(
            "render",
            Schedule::builder()
                .add_system(prepare_render())
                .add_system(render())
                .flush()
                .build(),
        )?;

        Ok(Test4World)
    }
}

impl GameUnloadWorld for Test4World {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test4 scene from the world");

        game.schedules.remove("render");
        game.schedules.remove("update");
        let _ = game.resources.remove::<TestScene>();
        let _ = game.resources.remove::<FirstPerson>();
        let _ = game.resources.remove::<Projection>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    pipeline: PipelineDependency,
    texture: TextureDependency,
    geometry: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    uniforms: Option<wgpu::Buffer>,
    bind_group: Option<PipelineBindGroup>,
}

impl TestScene {
    fn new(test: Test4) -> TestScene {
        TestScene {
            pipeline: PipelineDependency::new()
                .with_id(test.pipeline)
                .with_vertex_layout::<vertex::Pos3fTex2f>(),
            texture: TextureDependency::new().with_id(test.texture),
            geometry: None,
            uniforms: None,
            bind_group: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, projection: &Projection) {
        self.geometry.get_or_insert_with(|| {
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

        let uniforms = ViewProj::from(projection);

        match &self.uniforms {
            None => {
                self.uniforms = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&[uniforms]),
                    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                }));
            }
            Some(buffer) => {
                let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&[uniforms]),
                    usage: wgpu::BufferUsage::COPY_SRC,
                });

                encoder.copy_buffer_to_buffer(
                    &staging_buffer,
                    0,
                    &buffer,
                    0,
                    std::mem::size_of::<ViewProj>() as wgpu::BufferAddress,
                );
            }
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        resources: &RenderResources,
    ) {
        let pipelines = resources.pipelines.read();
        let textures = resources.textures.read();

        if let Ok(mut pass) = frame.begin_pass(encoder, FrameGraphDescriptor::SINGLE_PASS_NAME) {
            self.pipeline.or_state(pass.get_pipeline_state());
            if let (Some(ref geometry), Some(ref uniforms), Ok(Some(pipeline)), Ok(Some(texture))) = (
                self.geometry.as_ref(),
                self.uniforms.as_ref(),
                self.pipeline.request(&pipelines),
                self.texture.request(&textures),
            ) {
                if self.bind_group.is_none() {
                    self.bind_group = Some(pipeline.create_bind_groups(
                        device,
                        |_| unreachable!(),
                        |u| match u {
                            Uniform::Texture(TextureSemantic::Diffuse) => {
                                wgpu::BindingResource::TextureView(&texture.view)
                            }
                            Uniform::Sampler(TextureSemantic::Diffuse) => {
                                wgpu::BindingResource::Sampler(&texture.sampler)
                            }
                            Uniform::UniformBuffer(UniformSemantic::ViewProj) => wgpu::BindingResource::Buffer {
                                buffer: uniforms,
                                offset: 0,
                                size: None,
                            },
                            _ => unreachable!(),
                        },
                        |_| unreachable!(),
                    ));
                }

                pass.set_pipeline(&pipeline, self.bind_group.as_ref().unwrap());
                pass.set_vertex_buffer(0, geometry.0.slice(..));
                pass.set_index_buffer(geometry.1.slice(..));
                pass.draw_indexed(0..geometry.2, 0, 0..1);
            }
        }
    }
}

fn update_camera() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_camera")
        .read_resource::<InputMapper>()
        .read_resource::<CurrentInputState>()
        .write_resource::<FirstPerson>()
        .build(move |_, _, (mapper, input, camera), _| {
            let fps = mapper.as_input::<FirstPersonShooter>().unwrap();
            let dx = fps.x(&input);
            let dy = fps.y(&input);
            let dz = fps.z(&input);
            let dr = fps.roll(&input);
            let dp = fps.pitch(&input);
            let dw = fps.yaw(&input);
            camera.move_forward(dz * 0.01);
            camera.move_side(dx * 0.01);
            camera.move_up(dy * 0.01);
            camera.roll(dr * 0.01);
            camera.pitch(dp * 0.01);
            camera.yaw(dw * 0.01);
        })
}

fn bake_camera<C: Camera>() -> Box<dyn Schedulable> {
    SystemBuilder::new("bake_camera")
        .read_resource::<C>()
        .write_resource::<Projection>()
        .build(|_, _, (src, dst), _| {
            dst.set_camera::<C>(&src);
        })
}

fn prepare_render() -> Box<dyn Schedulable> {
    SystemBuilder::new("prepare_render")
        .read_resource::<Context>()
        .read_resource::<Projection>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, projection, scene), _| {
            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            scene.prepare(&context.device(), &mut encoder, &projection);

            context.queue().submit(Some(encoder.finish()));
        })
}

fn render() -> Box<dyn Schedulable> {
    SystemBuilder::new("render")
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
