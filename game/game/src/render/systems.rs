use crate::render::{Context, ShaderStore};
use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

pub fn update_shaders() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_shaders")
        .write_resource::<ShaderStore>()
        .write_resource::<Context>()
        .build(move |_, _, (shaders, context), _| {
            let mut shaders = shaders.write();
            //shaders.drain_unused();
            shaders.update(&context);
            shaders.finalize_requests();
        })
}
