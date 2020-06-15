use crate::components::camera::{Camera, Projection};
use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

pub fn bake_camera<C: Camera>() -> Box<dyn Schedulable> {
    SystemBuilder::new("bake_camera")
        .read_resource::<C>()
        .write_resource::<Projection>()
        .build(|_, _, (src, dst), _| {
            dst.set_camera::<C>(&src);
        })
}
