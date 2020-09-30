use crate::{
    input::{mappers, InputError, InputEvent, InputMapper},
    World,
};
use shine_input::{GuestureManager, InputManager, InputState};
use std::{
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
};

/// The input state for the current frame.
#[derive(Default)]
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

/// Wraper for the InputMapper to allow polymorhism.
pub struct WrapInputMapper {
    input: Box<dyn InputMapper>,
}

impl WrapInputMapper {
    pub fn wrap<I: InputMapper>(input: I) -> Self {
        Self { input: Box::new(input) }
    }
}

/// Handler for the inputs to prepare the state for the next frame.
#[derive(Default)]
pub struct InputHandler {
    state: InputState,
    manager: InputManager,
    guestures: GuestureManager,
}

impl InputHandler {
    pub fn inject_input(&mut self, mapper: &WrapInputMapper, event: InputEvent<'_>) {
        mapper.input.update_state(event, &mut self.state);
    }

    pub fn advance(&mut self, previous_state: &mut InputState) {
        self.manager
            .advance_states_with_guestures(previous_state, &mut self.state, &mut self.guestures);
    }
}

pub type InputFuture<'a, R> = Pin<Box<dyn Future<Output = R> + 'a>>;

pub trait InputPlugin {
    /// Add input handler plugin to the world
    fn add_input_plugin(&mut self) -> InputFuture<'_, Result<(), InputError>>;

    /// Remove input handler plugin from the world
    fn remove_input_plugin(&mut self) -> InputFuture<'_, Result<(), InputError>>;

    fn set_input_mapper<I: InputMapper>(&mut self, input: I) -> Result<(), InputError>;
    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), InputError>;
}

impl InputPlugin for World {
    fn add_input_plugin(&mut self) -> InputFuture<'_, Result<(), InputError>> {
        Box::pin(async move {
            log::info!("Adding input plugin");
            self.resources.insert(None, InputHandler::default());
            self.resources.insert(None, CurrentInputState::default());
            self.resources.insert(None, WrapInputMapper::wrap(mappers::Unmapped));
            Ok(())
        })
    }

    fn remove_input_plugin(&mut self) -> InputFuture<'_, Result<(), InputError>> {
        Box::pin(async move {
            log::info!("Removing input plugin");
            let _ = self.resources.remove::<InputHandler>(&None);
            let _ = self.resources.remove::<CurrentInputState>(&None);
            let _ = self.resources.remove::<WrapInputMapper>(&None);
            Ok(())
        })
    }

    fn set_input_mapper<I: InputMapper>(&mut self, input_mapper: I) -> Result<(), InputError> {
        let mut mapper = self.plugin_resource_mut::<WrapInputMapper>("input", &None)?;
        let mut handler = self.plugin_resource_mut::<InputHandler>("input", &None)?;
        let mut state = self.plugin_resource_mut::<CurrentInputState>("input", &None)?;

        *handler = InputHandler::default();
        *state = CurrentInputState::default();
        input_mapper.init_guestures(&mut handler.guestures);
        mapper.input = Box::new(input_mapper);
        Ok(())
    }

    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), InputError> {
        let mapper = self.plugin_resource::<WrapInputMapper>("input", &None)?;
        let mut handler = self.plugin_resource_mut::<InputHandler>("input", &None)?;

        handler.inject_input(&mapper, event.into());
        Ok(())
    }
}
