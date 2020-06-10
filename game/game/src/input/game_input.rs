use shine_input::{GuestureManager, InputManager, InputState};

#[derive(Debug)]
pub enum InputEvent<'e> {
    #[cfg(feature = "native")]
    Winit(&'e winit::event::Event<'e, ()>),

    NoEvent(&'e ()),
}

#[cfg(feature = "native")]
impl<'e> From<&'e winit::event::Event<'e, ()>> for InputEvent<'e> {
    fn from(e: &'e winit::event::Event<'e, ()>) -> InputEvent<'e> {
        InputEvent::Winit(e)
    }
}

pub trait GameInput: 'static + Send + Sync {
    fn init(&mut self, manager: &mut InputManager, guestures: &mut GuestureManager);
    fn update_state(&self, event: InputEvent<'_>, _state: &mut InputState);
}
