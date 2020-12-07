use crate::{
    app::AppError,
    input::{mappers, InputEvent, InputMapper},
    World,
};
use shine_ecs::resources::{Resource, ResourceRead, ResourceWrite};
use shine_input::{GuestureManager, InputManager, InputState};
use std::ops::{Deref, DerefMut};

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

impl World {
    pub fn input_plugin_name() -> &'static str {
        "input"
    }

    fn add_input_resource<T: Resource>(&mut self, resource: T) -> Result<(), AppError> {
        let _ = self
            .resources
            .insert(resource)
            .map_err(|err| AppError::plugin(Self::input_plugin_name(), err))?;
        Ok(())
    }

    fn get_input_resource<T: Resource>(&self) -> Result<ResourceRead<'_, T>, AppError> {
        Ok(self
            .resources
            .get::<T>()
            .map_err(|err| AppError::plugin_dependency(Self::input_plugin_name(), err))?)
    }

    fn get_mut_input_resource<T: Resource>(&self) -> Result<ResourceWrite<'_, T>, AppError> {
        Ok(self
            .resources
            .get_mut::<T>()
            .map_err(|err| AppError::plugin_dependency(Self::input_plugin_name(), err))?)
    }

    pub async fn add_input_plugin(&mut self) -> Result<(), AppError> {
        log::info!("Adding input plugin");
        self.add_input_resource(InputHandler::default())?;
        self.add_input_resource(CurrentInputState::default())?;
        self.add_input_resource(WrapInputMapper::wrap(mappers::Unmapped))?;
        Ok(())
    }

    pub async fn remove_input_plugin(&mut self) -> Result<(), AppError> {
        log::info!("Removing input plugin");
        let _ = self.resources.remove::<InputHandler>();
        let _ = self.resources.remove::<CurrentInputState>();
        let _ = self.resources.remove::<WrapInputMapper>();
        Ok(())
    }

    pub fn set_input_mapper<I: InputMapper>(&mut self, input_mapper: I) -> Result<(), AppError> {
        let mut mapper = self.get_mut_input_resource::<WrapInputMapper>()?;
        let mut handler = self.get_mut_input_resource::<InputHandler>()?;
        let mut state = self.get_mut_input_resource::<CurrentInputState>()?;

        *handler = InputHandler::default();
        *state = CurrentInputState::default();
        input_mapper.init_guestures(&mut handler.guestures);
        mapper.input = Box::new(input_mapper);
        Ok(())
    }

    pub fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), AppError> {
        let mapper = self.get_input_resource::<WrapInputMapper>()?;
        let mut handler = self.get_mut_input_resource::<InputHandler>()?;

        handler.inject_input(&mapper, event.into());
        Ok(())
    }
}
