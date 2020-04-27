use crate::GameError;
use shine_ecs::legion::systems::resource::Resources;
use shine_input::{InputManager, InputState};
use std::ops::{Deref, DerefMut};

pub mod systems;

/// The input state for the current frame.
pub struct CurrentInputState(InputState);

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

/// Handler for the inputs to prepare the state for the next frame.
pub struct InputHandler {
    state: InputState,
    manager: InputManager,
}

impl InputHandler {
    fn new() -> InputHandler {
        InputHandler {
            state: InputState::new(),
            manager: InputManager::new(),
        }
    }

    pub fn advance(&mut self, previous_state: &mut InputState) {
        self.manager.advance_states(previous_state, &mut self.state);
    }
}

/// Add required resource to handle inputs.
/// - *CurrentInputState* stores the input state for the current frame and thus should be Read only in systems.
/// - *InputHandler* handles the user inputs. As input arrives from a single thread, it should not be used from
/// systems generally and access it usually restricted to the main loop.
pub async fn add_input_system(resources: &mut Resources) -> Result<(), GameError> {
    log::info!("adding input system to the world");
    resources.insert(CurrentInputState(InputState::new()));
    resources.insert(InputHandler::new());
    Ok(())
}
