use crate::render::{Context, Shader, ShaderStore, Surface};
use shine_ecs::legion::{
    systems::{
        schedule::{Runnable, Schedulable},
        SystemBuilder,
    },
    thread_resources::WrapThreadResource,
};

pub fn prepare_context(thread_resources: WrapThreadResource) -> Box<dyn Runnable> {
    SystemBuilder::new("prepare_context")
        .write_resource::<Context>()
        .build_thread_local(move |_, _, context, _| {
            let thread_resources = thread_resources.get();
            let surface = thread_resources.get::<Surface>().unwrap();
            context.init_swap_chain(&*surface);
        })
}

pub fn update_shaders() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_shaders")
        .write_resource::<ShaderStore>()
        .write_resource::<Context>()
        .build(move |_, _, (shaders, _context), _| {
            let mut _shaders = shaders.write();
            //todo: compile shaders for device
        })
}
