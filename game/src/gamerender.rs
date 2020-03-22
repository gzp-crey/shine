use crate::input::InputManager;

pub struct GameRender {
    pub input: InputManager,
}

impl GameRender {
    pub fn new() -> Self {
        Self {
            input: InputManager::default(),
        }
    }
}
