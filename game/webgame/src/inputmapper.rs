use shine_input::{InputMapper, InputState};

#[derive(Debug)]
pub enum WebInputEvent {
    MouseDown(f32, f32),
}

pub struct WebInputMapper {}

impl WebInputMapper {
    pub fn new() -> WebInputMapper {
        WebInputMapper {}
    }
}

impl InputMapper for WebInputMapper {
    type InputEvent = WebInputEvent;

    fn update_state(&self, event: &WebInputEvent, _state: &mut InputState) {
        log::info!("{:?}", event);
    }
}
