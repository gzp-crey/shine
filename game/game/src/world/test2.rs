use crate::{
    assets::vertex::{self, Pos3fCol4f},
    render::{Context, Frame, PipelineDependency, PipelineStore, PipelineStoreRead, DEFAULT_PASS},
    world::{GameLoadWorld, GameUnloadWorld},
    GameError, GameView,
};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};
use wgpu::util::DeviceExt;

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
    pipeline: PipelineDependency,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
}

impl TestScene {
    fn new(test: Test2) -> TestScene {
        TestScene {
            pipeline: PipelineDependency::new()
                .with_id(test.pipeline)
                .with_vertex_layout::<vertex::Pos3fCol4f>(),
            buffers: None,
        }
    }

    fn prepare(&mut self, device: &wgpu::Device) {
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

    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &Frame, pipelines: &PipelineStoreRead<'_>) {
        //self.pipeline.or_state(frame.)
        if let (Some(buffers), Ok(Some(pipeline))) = (self.buffers.as_ref(), self.pipeline.request(pipelines)) {
            if let Ok(mut pass) = frame.create_pass(encoder, DEFAULT_PASS) {
                pass.set_pipeline(&pipeline.pipeline);
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
            scene.prepare(&context.device());

            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(&mut encoder, &*frame, &pipelines.read());

            frame.add_command(encoder.finish());
        })
}
