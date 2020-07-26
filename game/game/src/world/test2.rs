use crate::{
    assets::vertex::{self, Pos3fCol4f},
    render::{Context, Frame, PipelineKey, PipelineNamedId, PipelineStore, PipelineStoreRead},
    world::{GameLoadWorld, GameUnloadWorld},
    GameError, GameView,
};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};
use std::borrow::Cow;

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

/// Manage the lifecycle of the test
pub struct Test2World;

impl GameLoadWorld for Test2World {
    type Source = Test2;

    fn build(source: Test2, game: &mut GameView) -> Result<Test2World, GameError> {
        log::info!("Adding test2 scene to the world");

        game.resources.insert(TestScene::new(source));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(Test2World)
    }
}

impl GameUnloadWorld for Test2World {
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

    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &Frame, pipelines: &mut PipelineStoreRead<'_>) {
        if let (Some(buffers), Some(pipeline)) = (self.buffers.as_ref(), self.pipeline.get(pipelines).pipeline_buffer())
        {
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
            pass.set_vertex_buffer(0, buffers.0.slice(..));
            pass.set_index_buffer(buffers.1.slice(..));
            pass.draw_indexed(0..buffers.2, 0, 0..1);
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
            scene.prepare(&context.device());

            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(&mut encoder, &*frame, &mut pipelines.read());

            frame.add_command(encoder.finish());
        })
}
