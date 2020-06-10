use crate::input::{GameInput, InputEvent, InputManager};
use shine_input::{GuestureManager, InputState};

pub struct FPSInputMapper {}

impl FPSInputMapper {
    pub fn new() -> FPSInputMapper {
        FPSInputMapper {}
    }
}

impl GameInput for FPSInputMapper {
    fn init(&mut self, manager: &mut InputManager, guestures: &mut GuestureManager) {}

    fn update_state<'e>(&self, event: InputEvent<'e>, _state: &mut InputState) {
        log::info!("{:?}", event);
    }
}
