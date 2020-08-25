use crate::{
    assets::{vertex, TextureSemantic, Uniform},
    render::{Context, Frame, PipelineDependency, PipelineStore, PipelineStoreRead, RenderPlugin},
    world::{GameLoadWorld, GameUnloadWorld},
    GameError, GameView,
};
use serde::{Deserialize, Serialize};
use shine_ecs::legion::{
    systems::schedule::{Schedulable, Schedule},
    systems::SystemBuilder,
};

/// Serialized test
#[derive(Debug, Serialize, Deserialize)]
pub struct Test5 {
    pub frame_graph: String,
    pub scene_pipeline: String,
    pub present_pipeline: String,
}

/// Manage the lifecycle of the test
pub struct Test5World;

impl GameLoadWorld for Test5World {
    type Source = Test5;

    fn build(source: Test5, game: &mut GameView) -> Result<Test5World, GameError> {
        log::info!("Adding test5 scene to the world");

        game.set_frame_graph(Some(source.frame_graph.clone()));
        game.resources.insert(TestScene::new(source));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(Test5World)
    }
}

impl GameUnloadWorld for Test5World {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test5 scene from the world");

        game.set_frame_graph(None);
        game.schedules.remove("render");
        let _ = game.resources.remove::<TestScene>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    scene_pipeline: PipelineDependency,
    present_pipeline: PipelineDependency,
}

impl TestScene {
    fn new(test: Test5) -> TestScene {
        TestScene {
            scene_pipeline: PipelineDependency::new()
                .with_id(test.scene_pipeline)
                .with_vertex_layout::<vertex::Null>(),
            present_pipeline: PipelineDependency::new()
                .with_id(test.present_pipeline)
                .with_vertex_layout::<vertex::Null>(),
        }
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        pipelines: &PipelineStoreRead<'_>,
    ) {
        unimplemented!()
        /*if let (Some(scene_pipeline), Some(present_pipeline)) = (
            self.scene_pipeline.get(pipelines).pipeline_buffer(),
            self.present_pipeline.get(pipelines).pipeline_buffer(),
        ) {
            if let Ok((mut pass, _)) = frame.create_pass(encoder, "scene") {
                pass.set_pipeline(&scene_pipeline.pipeline);
                pass.draw(0..3, 0..1);
            }

            if let Ok(textures) = frame.pass_textures("present") {
                let bind_group = present_pipeline
                    .create_bind_group(GLOBAL_UNIFORMS, device, |u| match u {
                        Uniform::Texture(TextureSemantic::Frame(name)) => {
                            wgpu::BindingResource::TextureView(&textures.textures[0].0.render_target.view)
                        }
                        Uniform::Sampler(TextureSemantic::Frame(name)) => {
                            wgpu::BindingResource::Sampler(&textures.textures[0].1)
                        }
                        _ => unreachable!(),
                    })
                    .unwrap();

                if let Ok((mut pass, _)) = frame.create_pass(encoder, "present") {
                    pass.set_pipeline(&present_pipeline.pipeline);
                    pass.set_bind_group(GLOBAL_UNIFORMS, &bind_group, &[]);
                    pass.draw(0..3, 0..1);
                }
            }
        }*/
    }
}

fn render_test() -> Box<dyn Schedulable> {
    SystemBuilder::new("test_render")
        .read_resource::<Context>()
        .read_resource::<Frame>()
        .read_resource::<PipelineStore>()
        .write_resource::<TestScene>()
        .build(move |_, _, (context, frame, pipelines, scene), _| {
            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(context.device(), &mut encoder, &frame, &pipelines.read());
            frame.add_command(encoder.finish());
        })
}
