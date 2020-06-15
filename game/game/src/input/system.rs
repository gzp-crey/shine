use crate::input::{mapper, CurrentInputState, GameInput, InputEvent, InputHandler, InputMapper};
use crate::{GameError, GameView};
use shine_ecs::legion::query::{Read, Write};
use shine_ecs::legion::systems::resource::ResourceSet;

pub trait InputSystem {
    fn add_input_system(&mut self) -> Result<(), GameError>;
    fn remove_input_system(&mut self) -> Result<(), GameError>;

    fn set_input<I: GameInput>(&mut self, input: I) -> Result<(), GameError>;
    fn inject_input<'e, E: Into<InputEvent<'e>>>(&mut self, event: E) -> Result<(), GameError>;
}

impl InputSystem for GameView {
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
