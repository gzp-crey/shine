use crate::assets::vertex;
use crate::render::{Context, Frame, PipelineKey, PipelineNamedId, PipelineStore, PipelineStoreRead};
use crate::world::{GameLoadWorld, GameUnloadWorld};
use crate::{GameError, GameView};
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

        game.resources.insert(TestScene::new(source));

        let render = Schedule::builder().add_system(render_test()).flush().build();
        game.schedules.insert("render", render)?;

        Ok(Test5World)
    }
}

impl GameUnloadWorld for Test5World {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError> {
        log::info!("Removing test5 scene from the world");

        game.schedules.remove("render");
        let _ = game.resources.remove::<TestScene>();

        Ok(())
    }
}

/// Resources for the test
struct TestScene {
    scene_pipeline: PipelineNamedId,
    present_pipeline: PipelineNamedId,
}

impl TestScene {
    fn new(test: Test5) -> TestScene {
        TestScene {
            scene_pipeline: PipelineNamedId::from_key(PipelineKey::new::<vertex::Null>(&test.scene_pipeline)),
            present_pipeline: PipelineNamedId::from_key(PipelineKey::new::<vertex::Null>(&test.present_pipeline)),
        }
    }

    fn render(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &Frame, pipelines: &PipelineStoreRead<'_>) {
        if let (Some(scene_pipeline), Some(present_pipeline)) = (
            self.scene_pipeline.get(pipelines).pipeline_buffer(),
            self.present_pipeline.get(pipelines).pipeline_buffer(),
        ) {
            {
                let (mut pass, _) = frame.create_pass(encoder, "scene");
                pass.set_pipeline(&scene_pipeline.pipeline);
                pass.draw(0..3, 0..1);
            }

            {
                let (mut pass, _) = frame.create_pass(encoder, "present");
                pass.set_pipeline(&present_pipeline.pipeline);
                pass.draw(0..3, 0..1);
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
            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            scene.render(&mut encoder, &frame, &pipelines.read());
            frame.add_command(encoder.finish());
        })
}
