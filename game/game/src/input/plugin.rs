use crate::{
    input::{mapper, GameInput, InputEvent},
    GameView,
};
use shine_input::{GuestureManager, InputManager, InputState};
use std::{
    any::Any,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
};

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

#[derive(Debug)]
pub struct InputError;

pub type InputFuture<'a, R> = Pin<Box<dyn Future<Output = R> + 'a>>;

pub trait InputPlugin {
    /// Add input handler plugin to the world
    fn add_input_plugin<'a>(&'a mut self) -> InputFuture<'a, Result<(), InputError>>;

    /// Remove input handler plugin from the world
    fn remove_input_plugin<'a>(&'a mut self) -> InputFuture<'a, Result<(), InputError>>;

    fn set_input<I: GameInput>(&mut self, input: I) -> Result<(), InputError>;
    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), InputError>;
}

impl InputPlugin for GameView {
    fn add_input_plugin<'a>(&'a mut self) -> InputFuture<'a, Result<(), InputError>> {
        Box::pin(async move {
            log::info!("Adding input plugin");
            self.resources.insert(None, InputHandler::new());
            self.resources.insert(None, CurrentInputState::new());
            self.resources.insert(None, InputMapper::new(mapper::Unmapped));
            Ok(())
        })
    }

    fn remove_input_plugin<'a>(&'a mut self) -> InputFuture<'a, Result<(), InputError>> {
        Box::pin(async move {
            log::info!("Removing input plugin");
            let _ = self.resources.remove::<InputHandler>(None);
            let _ = self.resources.remove::<CurrentInputState>(None);
            let _ = self.resources.remove::<InputMapper>(None);
            Ok(())
        })
    }

    fn set_input<I: GameInput>(&mut self, input: I) -> Result<(), InputError> {
        let mut mapper = self.resources.get_mut::<InputMapper>(None).unwrap();
        let mut handler = self.resources.get_mut::<InputHandler>(None).unwrap();
        let mut state = self.resources.get_mut::<CurrentInputState>(None).unwrap();

        *handler = InputHandler::new();
        *state = CurrentInputState::new();
        input.init_guestures(&mut handler.guestures);
        mapper.input = Box::new(input);
        Ok(())
    }

    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), InputError> {
        let mapper = self.resources.get::<InputMapper>(None).unwrap();
        let mut handler = self.resources.get_mut::<InputHandler>(None).unwrap();

        handler.inject_input(&mapper, event.into());
        Ok(())
    }
}
