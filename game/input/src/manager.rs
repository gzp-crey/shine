use crate::{GuestureManager, InputState};
use std::mem;

#[cfg(feature = "native")]
use std::time::SystemTime;
#[cfg(feature = "wasm")]
use wasm_timer::SystemTime;

pub struct InputManager {
    time: u128,
}

impl InputManager {
    fn now() -> u128 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
    }

    pub fn new() -> InputManager {
        InputManager { time: 0 }
    }

    /// Prepare for the next input frame.
    pub fn advance_states(&mut self, previous: &mut InputState, current: &mut InputState) {
        self.advance_states_with(previous, current, |_, _| {});
    }

    /// Prepare for the next input frame. The current state is made the previous state
    /// and the current state is prepared to accept new inputs.
    /// An update function can be also given to handle other state transitions, ex guestures.
    pub fn advance_states_with<F: FnOnce(&mut InputState, &mut InputState)>(
        &mut self,
        previous: &mut InputState,
        current: &mut InputState,
        on_update: F,
    ) {
        self.time = Self::now();
        //fixme: state time is off by one frame, current time is the end of the prev update
        on_update(previous, current);
        mem::swap(previous, current);
        current.init_from(previous, self.time);
    }

    /// Prepare for the next input frame.
    pub fn advance_states_with_guestures(
        &mut self,
        previous: &mut InputState,
        current: &mut InputState,
        guestures: &mut GuestureManager,
    ) {
        self.advance_states_with(previous, current, |p, c| guestures.process_guestures(p, c));
    }
}

impl Default for InputManager {
    fn default() -> InputManager {
        InputManager::new()
    }
}
