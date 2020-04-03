use crate::input::{CurrentInputState, InputHandler};
use shine_ecs::legion::{schedule::Schedulable, system::SystemBuilder};

pub fn advance_input_states() -> Box<dyn Schedulable> {
    SystemBuilder::new("advance_input_states")
        .write_resource::<CurrentInputState>()
        .write_resource::<InputHandler>()
        .build(|_, _, (prev, handler), _| {
            handler.advance(prev);
        })
}
