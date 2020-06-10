use crate::GameError;
use shine_ecs::legion::systems::resource::Resources;
use shine_input::{GuestureManager, InputManager, InputState};
use std::ops::{Deref, DerefMut};

mod game_input;
pub use self::game_input::*;
mod fps_input_mapper;
pub use self::fps_input_mapper::*;

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
    guestures: GuestureManager,
    input: Box<dyn GameInput>,
}

impl InputHandler {
    fn new<I: GameInput>(mut input: I) -> InputHandler {
        let mut manager = InputManager::new();
        let mut guestures = GuestureManager::new();
        input.init(&mut manager, &mut guestures);
        InputHandler {
            state: InputState::new(),
            manager,
            guestures,
            input: Box::new(input),
        }
    }

    pub fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) {
        self.input.update_state(event.into(), &mut self.state);
    }

    pub fn advance(&mut self, previous_state: &mut InputState) {
        self.manager
            .advance_states_with_guestures(previous_state, &mut self.state, &mut self.guestures);
    }
}

/// Add required resource to handle inputs.
/// - *CurrentInputState* stores the input state for the current frame and thus should be Read only in systems.
/// - *InputHandler* handles the user inputs. As input arrives from a single thread, it should not be used from
/// systems generally and access it usually restricted to the main loop.
pub async fn add_input_system<I: GameInput>(resources: &mut Resources, input: I) -> Result<(), GameError> {
    log::info!("adding input system to the world");
    resources.insert(CurrentInputState(InputState::new()));
    resources.insert(InputHandler::new(input));
    Ok(())
}

pub async fn remove_input_system(resources: &mut Resources) -> Result<(), GameError> {
    log::info!("removing input system to the world");
    let _ = resources.remove::<InputHandler>();
    let _ = resources.remove::<CurrentInputState>();
    Ok(())
}
