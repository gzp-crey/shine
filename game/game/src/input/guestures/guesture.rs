use crate::input::{InputId, InputState};

pub trait Guesture: Send + Sync {
    fn inputs(&self) -> Vec<InputId>;
    fn outputs(&self) -> Vec<InputId>;
    fn on_update(&mut self, prev_state: &InputState, state: &mut InputState);
}
