use crate::render::{Context, Frame, PipelineIndex, PipelineStore, PipelineStoreRead, PipelineKey, VertexNull};
use crate::utils::runtime::Runtime;
use crate::{Config, GameError};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::{resource::Resources, SystemBuilder},
    world::World,
};
use wgpu;

struct TestScene {
    pipeline: Option<PipelineIndex>,
}

impl TestScene {
    fn new() -> TestScene {
        TestScene { pipeline: None }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'_, '_>,
        pipelines: &mut PipelineStoreRead<'_>,
    ) {
        let pipeline = self.pipeline.get_or_insert_with(|| {
            pipelines.get_or_add_blocking(&PipelineKey::new::<VertexNull>(
                "2ac6/7657415604b23eb7ab7b32cc86d6c22b8b319ce813b53f75ad6ad3d6385b.pl",
            ))
        });

        let pipeline = pipelines.at(pipeline);
        //let pipeline = &pipelines[pipeline];
        if let Some(mut pipeline) = pipeline.bind(encoder, pass_descriptor) {
            pipeline.draw(0..3, 0..1);
        }
    }
}

/// Add required resource for the test scene
pub async fn add_test_scene(
    _config: &Config,
    resources: &mut Resources,
    _world: &mut World,
    _runtime: &mut Runtime,
) -> Result<(), GameError> {
    log::info!("adding test scene to the world");

    resources.insert(TestScene::new());

    Ok(())
}

fn render_test() -> Box<dyn Schedulable> {
    SystemBuilder::new("test_render")
        .read_resource::<Context>()
        .read_resource::<Frame>()
        .write_resource::<PipelineStore>()
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

                scene.render(&mut encoder, &pass_descriptor, &mut pipelines);
            }

            frame.add_command(encoder.finish());
        })
}

pub fn create_schedule() -> Schedule {
    Schedule::builder().add_system(render_test()).flush().build()
}
