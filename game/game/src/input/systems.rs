use crate::input::{CurrentInputState, InputHandler};
use shine_ecs::ecs::resources::ResMut;

pub fn advance_input_states(mut prev_states: ResMut<CurrentInputState>, mut handler: ResMut<InputHandler>) {
    handler.advance(&mut prev_states);
}
