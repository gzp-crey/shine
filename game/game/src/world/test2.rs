use crate::assets::vertex::{self, Pos3fCol4f};
use crate::render::{Context, Frame, PipelineKey, PipelineNamedId, PipelineStore, PipelineStoreRead};
use crate::world::{GameWorld, GameWorldBuilder};
use crate::{GameError, GameView};
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

// const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4]; workaround for Buffers that are mapped at creation have to be aligned to COPY_BUFFER_ALIGNMENT'
const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, 0];
const INDEX_COUNT: usize = 9;

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test2 {
    pub pipeline: String,
}

impl GameWorldBuilder for Test2 {
    type World = TestWorld;

    fn build(self, game: &mut GameView) -> Result<TestWorld, GameError> {
        log::info!("Adding test2 scene to the world");

        game.resources.insert(TestScene::new(self));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(TestWorld)
    }
}

/// Manage the lifecycle of the test
pub struct TestWorld;

impl GameWorld for TestWorld {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test2 scene from the world");

        game.schedules.remove("render");
        let _ = game.resources.remove::<TestScene>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    pipeline: PipelineNamedId,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
}

impl TestScene {
    fn new(test: Test2) -> TestScene {
        TestScene {
            pipeline: PipelineNamedId::from_key(PipelineKey::new::<vertex::Pos3fCol4f>(&test.pipeline)),
            buffers: None,
        }
    }

    fn prepare(&mut self, device: &wgpu::Device) {
        self.buffers.get_or_insert_with(|| {
            log::trace!("creating buffers");
            let v = device.create_buffer_with_data(bytemuck::cast_slice(VERTICES), wgpu::BufferUsage::VERTEX);
            log::trace!("creating buffers2");
            let i = device.create_buffer_with_data(bytemuck::cast_slice(INDICES), wgpu::BufferUsage::INDEX);
            log::trace!("creating buffers3");
            (v, i, INDEX_COUNT/*INDICES.len()*/ as u32)
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
