use shine_input::{GuestureManager, InputManager, InputState};
use std::any::Any;
use std::ops::{Deref, DerefMut};

mod game_input;
pub use self::game_input::*;
mod system;
pub use self::system::*;

pub mod mapper;
pub mod systems;

/// The input state for the current frame.
pub struct CurrentInputState(InputState);

impl CurrentInputState {
    pub fn new() -> CurrentInputState {
        CurrentInputState(InputState::new())
    }
}

impl Deref for CurrentInputState {
    type Target = InputState;

    fn deref(&self) -> &InputState {
        &self.0
    }
}

impl DerefMut for CurrentInputState {
    fn deref_mut(&mut self) -> &mut InputState {
        &mut self.0
    }
}

/// Convert user inputs into InputState changes
pub struct InputMapper {
    input: Box<dyn GameInput>,
}

impl InputMapper {
    pub fn new<I: GameInput>(input: I) -> InputMapper {
        InputMapper { input: Box::new(input) }
    }

    pub fn as_input<I: GameInput>(&self) -> Option<&I> {
        Any::downcast_ref::<I>(self.input.as_any())
    }

    pub fn as_input_mut<I: GameInput>(&mut self) -> Option<&mut I> {
        Any::downcast_mut::<I>(self.input.as_any_mut())
    }
}

/// Handler for the inputs to prepare the state for the next frame.
pub struct InputHandler {
    state: InputState,
    manager: InputManager,
    guestures: GuestureManager,
}

impl InputHandler {
    fn new() -> InputHandler {
        InputHandler {
            state: InputState::new(),
            manager: InputManager::new(),
            guestures: GuestureManager::new(),
        }
    }

    pub fn inject_input(&mut self, mapper: &InputMapper, event: InputEvent<'_>) {
        mapper.input.update_state(event, &mut self.state);
    }

    pub fn advance(&mut self, previous_state: &mut InputState) {
        self.manager
            .advance_states_with_guestures(previous_state, &mut self.state, &mut self.guestures);
    }
}
