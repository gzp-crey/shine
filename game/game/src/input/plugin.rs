use crate::{
    app::{AppError, Plugin, PluginFuture},
    input::{mappers, InputEvent, InputMapper},
    World,
};
use shine_input::{GuestureManager, InputManager, InputState};
use std::{
    borrow::Cow,
    error::Error as StdError,
    ops::{Deref, DerefMut},
};

pub const INPUT_PLUGIN_NAME: &str = "input";

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

pub struct InputPlugin;

fn into_plugin_err<E: 'static + StdError>(error: E) -> AppError {
    AppError::game(INPUT_PLUGIN_NAME, error)
}

impl Plugin for InputPlugin {
    fn name() -> Cow<'static, str> {
        INPUT_PLUGIN_NAME.into()
    }

    fn init(self, world: &mut World) -> PluginFuture<()> {
        Box::pin(async move {
            world
                .resources
                .register_with_instance(InputHandler::default())
                .map_err(into_plugin_err)?;
            world
                .resources
                .register_with_instance(CurrentInputState::default())
                .map_err(into_plugin_err)?;
            world
                .resources
                .register_with_instance(WrapInputMapper::wrap(mappers::Unmapped))
                .map_err(into_plugin_err)?;
            Ok(())
        })
    }

    fn deinit(world: &mut World) -> PluginFuture<()> {
        Box::pin(async move {
            let _ = world.resources.unregister::<InputHandler>();
            let _ = world.resources.unregister::<CurrentInputState>();
            let _ = world.resources.unregister::<WrapInputMapper>();
            Ok(())
        })
    }
}

pub trait InputWorld {
    fn set_input_mapper<I: InputMapper>(&mut self, input_mapper: I) -> Result<(), AppError>;
    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), AppError>;
}

impl InputWorld for World {
    fn set_input_mapper<I: InputMapper>(&mut self, input_mapper: I) -> Result<(), AppError> {
        let mut mapper = self.resources.get_mut::<WrapInputMapper>().map_err(into_plugin_err)?;
        let mut handler = self.resources.get_mut::<InputHandler>().map_err(into_plugin_err)?;
        let mut state = self.resources.get_mut::<CurrentInputState>().map_err(into_plugin_err)?;

        *handler = InputHandler::default();
        *state = CurrentInputState::default();
        input_mapper.init_guestures(&mut handler.guestures);
        mapper.input = Box::new(input_mapper);
        Ok(())
    }

    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), AppError> {
        let mapper = self.resources.get::<WrapInputMapper>().map_err(into_plugin_err)?;
        let mut handler = self.resources.get_mut::<InputHandler>().map_err(into_plugin_err)?;

        handler.inject_input(&mapper, event.into());
        Ok(())
    }
}
