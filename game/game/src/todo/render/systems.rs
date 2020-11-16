use crate::render::{Context, RenderResources};
use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

pub fn update_resources() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_shaders")
        .read_resource::<Context>()
        .write_resource::<RenderResources>()
        .build(move |_, _, (context, resources), _| {
            resources.update(&*context);
        })
}

pub fn gc_resources() -> Box<dyn Schedulable> {
    SystemBuilder::new("gc_shaders")
        .write_resource::<RenderResources>()
        .build(move |_, _, resources, _| {
            resources.gc();
        })
}
