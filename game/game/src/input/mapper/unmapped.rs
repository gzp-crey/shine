use crate::input::{GameInput, InputEvent};
use shine_input::{GuestureManager, InputState};
use std::any::Any;

/// Default game input, that ignores ever user input
pub struct Unmapped;

impl GameInput for Unmapped {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn init_guestures(&self, _guestures: &mut GuestureManager) {
        /* nop */
    }

    fn update_state(&self, _event: InputEvent<'_>, _state: &mut InputState) {
        /* nop */
    }
}
