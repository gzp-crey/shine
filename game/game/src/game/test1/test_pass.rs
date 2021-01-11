use crate::{
    app::AppError,
    assets::vertex,
    game::test1::into_game_err,
    render::{FrameTarget, PipelineKey},
};
use shine_ecs::{
    resources::{ResourceId, Resources},
    scheduler::{ResourceClaims, System, SystemName, TaskGroup},
    ECSError,
};

pub struct TestPass {
    pipeline_key: PipelineKey,
    pipeline_id: Option<ResourceId>,
    claims: ResourceClaims,
}

impl TestPass {
    pub fn new(pipeline: String) -> Box<TestPass> {
        Box::new(TestPass {
            pipeline_key: PipelineKey::new::<vertex::Null>(pipeline, Default::default()),
            pipeline_id: None,
            claims: Default::default(),
        })
    }

    pub fn set_render_state(&mut self, target: &FrameTarget) -> Result<(), AppError> {
        let pipeline_states = target.get_render_states();
        if self.pipeline_key.render_state != pipeline_states {
            self.pipeline_key.render_state = pipeline_states;
            self.pipeline_id = None;
            self.pipeline_id = Some(ResourceId::from_object(&self.pipeline_key).map_err(into_game_err)?);
        }

        Ok(())
    }
}

impl System for TestPass {
    fn debug_name(&self) -> &str {
        "TestPass"
    }

    fn name(&self) -> Option<&SystemName> {
        None
    }

    /// Resources claims. Claim shall not change once scheduler execution was started.
    fn resource_claims(&self) -> &ResourceClaims {
        &self.claims
    }

    fn run(&mut self, resources: &Resources) -> Result<TaskGroup, ECSError> {
        unimplemented!();
        //Ok(TaskGroup::default())
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
