use shine_input::{GuestureManager, InputState};
use std::any::Any;

#[derive(Debug)]
pub enum InputEvent<'e> {
    #[cfg(feature = "native")]
    Winit(&'e winit::event::KeyboardInput),

    NoEvent(&'e ()),
}

#[cfg(feature = "native")]
impl<'e> From<&'e winit::event::KeyboardInput> for InputEvent<'e> {
    fn from(e: &'e winit::event::KeyboardInput) -> InputEvent<'e> {
        InputEvent::Winit(e)
    }
}

pub trait GameInput: 'static + Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn init_guestures(&self, guestures: &mut GuestureManager);
    fn update_state(&self, event: InputEvent<'_>, _state: &mut InputState);
}
