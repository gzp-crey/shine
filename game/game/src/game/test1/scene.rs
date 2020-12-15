use crate::{assets::vertex, game::test1::Test1};

/// Resources for the test
pub struct TestScene {
    //pipeline: PipelineDependency,
//bind_group: Option<PipelineBindGroup>,
}

impl TestScene {
    pub fn new(test: &Test1) -> TestScene {
        TestScene {
  //          pipeline: PipelineDependency::default()
                //.with_id(test.pipeline.clone())
                //.with_vertex_layout::<vertex::Null>(),
            //bind_group: None,
        }
    }
}
/*
fn render_system(claim: RenderTargetClaim) -> Box<dyn System> {
    render.system().with_claim::<RenderTargetRes>(claim)
}

fn render(context: Res<Context>, resources: Res<RenderResources>, target: RenderTargetRes, scene: ResMut<TestScene>) {
    log::error!("render");
    let device = context.device();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    /*let pipelines = resources.pipelines.read();
    match frame.begin_pass(&mut encoder, FrameGraphDescriptor::SINGLE_PASS_NAME) {
        Ok(mut pass) => {
            let scene = &mut *scene;
            scene.pipeline.or_state(pass.get_pipeline_state());
            if let Ok(Some(pipeline)) = scene.pipeline.request(&pipelines) {
                if scene.bind_group.is_none() {
                    log::error!("precre");
                    scene.bind_group = Some(pipeline.create_bind_groups(
                        device,
                        |_| unreachable!(),
                        |_| unreachable!(),
                        |_| unreachable!(),
                    ));
                    log::error!("cre");
                }

                pass.set_pipeline(&pipeline, scene.bind_group.as_ref().unwrap());
                log::error!("predraw");
                pass.draw(0..3, 0..1);
                log::error!("draw");
            }
        }
        Err(err) => {
            log::error!("render error: {:?}", err);
        }
    }*/

    log::error!("pre command");
    context.add_command(encoder.finish());
    log::error!("end command");
}
*/
