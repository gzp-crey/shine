use crate::render::{
    vertex, Context, Frame, ModelIndex, ModelStore, ModelStoreRead, PipelineIndex, PipelineKey, PipelineStore,
    PipelineStoreRead,
};
use crate::GameError;
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::{resource::Resources, SystemBuilder},
};

struct TestScene {
    pipeline: Option<PipelineIndex>,
    model: Option<ModelIndex>,
}

impl TestScene {
    fn new() -> TestScene {
        TestScene {
            pipeline: None,
            model: None,
        }
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'_, '_>,
        pipelines: &mut PipelineStoreRead<'_>,
        models: &mut ModelStoreRead<'_>,
    ) {
        let pipeline = self.pipeline.get_or_insert_with(|| {
            pipelines.get_or_add_blocking(&PipelineKey::new::<vertex::Null>(
                "fe89/b2406e97285d2964831bc4914375778a9051cb3320bab7f5fc92444ce1ed.pl",
            ))
        });

        let model = self.model.get_or_insert_with(|| {
            models.get_or_add_blocking(
                &"8070/7e46ce08f84d3235d50029105864ea734535afccb037ac813173a4c5f968.glb".to_owned(),
            )
        });

        let pipeline = pipelines.at(pipeline);
        //let pipeline = &pipelines[pipeline];
        if let Some(mut pipeline) = pipeline.bind(encoder, pass_descriptor) {
            pipeline.draw(0..3, 0..1);
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
        .read_resource::<ModelStore>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, frame, pipelines, models, scene), _| {
            let mut pipelines = pipelines.read();
            let mut models = models.read();

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
                scene.render(&mut encoder, &pass_descriptor, &mut pipelines, &mut models);
            }

            frame.add_command(encoder.finish());
        })
}

pub fn create_schedule() -> Schedule {
    Schedule::builder().add_system(render_test()).flush().build()
}
