use crate::input::{CurrentInputState, InputHandler};
use shine_ecs::legion::{
    systems::{schedule::{Runnable, Schedulable}, SystemBuilder},
    thread_resources::{ThreadResources, WrapThreadResource},
};

pub fn prepare_context(wrap_thread_resources: WrapThreadResource) -> Box<dyn Runnable> {
    SystemBuilder::new("prepare_context")
        .build_thread_local(move |_, _, _, _| {
            let thread_resources = wrap_thread_resources.get();
        })
}
