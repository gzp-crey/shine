use crate::input::{mapper, GameInput, InputEvent};
use crate::{GameError, GameView};
use shine_ecs::legion::query::{Read, Write};
use shine_ecs::legion::systems::resource::ResourceSet;
use shine_input::{GuestureManager, InputManager, InputState};
use std::any::Any;
use std::ops::{Deref, DerefMut};

/// The input state for the current frame.
pub struct CurrentInputState(InputState);

impl CurrentInputState {
    pub fn new() -> CurrentInputState {
        CurrentInputState(InputState::new())
    }
}

impl Default for CurrentInputState {
    fn default() -> Self {
        Self::new()
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

pub trait InputPlugin {
    fn add_input_system(&mut self) -> Result<(), GameError>;
    fn remove_input_system(&mut self) -> Result<(), GameError>;

    fn set_input<I: GameInput>(&mut self, input: I) -> Result<(), GameError>;
    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), GameError>;
}

impl InputPlugin for GameView {
    fn add_input_system(&mut self) -> Result<(), GameError> {
        log::info!("adding input system to the world");
        self.resources.insert(InputHandler::new());
        self.resources.insert(CurrentInputState::new());
        self.resources.insert(InputMapper::new(mapper::Unmapped));
        Ok(())
    }

    fn remove_input_system(&mut self) -> Result<(), GameError> {
        log::info!("removing input system from the world");
        let _ = self.resources.remove::<InputHandler>();
        let _ = self.resources.remove::<CurrentInputState>();
        let _ = self.resources.remove::<InputMapper>();
        Ok(())
    }

    fn set_input<I: GameInput>(&mut self, input: I) -> Result<(), GameError> {
        let (mut mapper, mut handler, mut state) =
            <(Write<InputMapper>, Write<InputHandler>, Write<CurrentInputState>)>::fetch_mut(&mut self.resources);
        *handler = InputHandler::new();
        *state = CurrentInputState::new();
        input.init_guestures(&mut handler.guestures);
        mapper.input = Box::new(input);
        Ok(())
    }

    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), GameError> {
        let (mapper, mut handler) = <(Read<InputMapper>, Write<InputHandler>)>::fetch_mut(&mut self.resources);
        handler.inject_input(&mapper, event.into());
        Ok(())
    }
}
