use crate::input::{CurrentInputState, InputHandler};
use shine_ecs::shred::{System, WriteExpect};
use shine_input::InputManager;

/// Task to perform the update of the previous and current input states.
pub struct AdvanceInputState {}

pub fn advance_input_states() -> AdvanceInputState {
    AdvanceInputState {}
}

impl<'a> System<'a> for AdvanceInputState {
    type SystemData = (WriteExpect<'a, CurrentInputState>, WriteExpect<'a, InputHandler>);

    fn run(&mut self, data: Self::SystemData) {
        let (mut prev, mut handler) = data;

        handler.advance(&mut prev);
    }
}
