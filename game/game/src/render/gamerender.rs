use crate::input::InputManager;
use crate::render::Context;
use crate::GameError;

pub struct GameRender {
    pub input: InputManager,
    pub context: Context,
}

impl GameRender {
    pub async fn new(surface: wgpu::Surface) -> Result<GameRender, GameError> {
        Ok(GameRender {
            input: InputManager::default(),
            context: Context::new(surface).await?,
        })
    }
}
