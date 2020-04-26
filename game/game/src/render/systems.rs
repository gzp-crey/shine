use crate::render::{Context, PipelineStore, ShaderStore};
use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

pub fn update_shaders() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_shaders")
        .read_resource::<Context>()
        .write_resource::<ShaderStore>()
        .build(move |_, _, (context, shaders), _| {
            //log::info!("shader");
            let mut shaders = shaders.write();
            //shaders.drain_unused();
            let context: &Context = &*context;
            shaders.update(&mut (context,));
            shaders.finalize_requests();
        })
}

pub fn update_pipeline() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_pipeline")
        .read_resource::<Context>()
        .read_resource::<ShaderStore>()
        .write_resource::<PipelineStore>()
        .build(move |_, _, (context, shaders, pipeline), _| {
            //log::info!("pipeline");
            let mut pipeline = pipeline.write();
            let context: &Context = &*context;
            let shaders: &ShaderStore = &*shaders;
            //shaders.drain_unused();
            pipeline.update(&mut (context, shaders));
            pipeline.finalize_requests();
        })
}
