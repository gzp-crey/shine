mod error;
pub use self::error::*;
mod config;
pub use self::config::*;
mod game_lifecycle;
pub use self::game_lifecycle::*;

use crate::World;

#[derive(Default)]
pub struct App {
    pub world: World,
    game_loader: Option<Box<dyn GameLifecycle>>,
}

impl App {
    pub async fn init_game<S: GameSource>(&mut self, game: S) -> Result<(), AppError> {
        self.deinit_game().await?;
        let mut game = game.build()?;
        game.create(&mut self.world).await?;
        self.game_loader = Some(game);
        Ok(())
    }

    pub async fn deinit_game(&mut self) -> Result<(), AppError> {
        if let Some(mut game_loader) = self.game_loader.take() {
            game_loader.destroy(&mut self.world).await?;
        }
        Ok(())
    }

    pub async fn reload_game(&mut self) -> Result<(), AppError> {
        if let Some(game_loader) = &mut self.game_loader {
            game_loader.destroy(&mut self.world).await?;
            game_loader.create(&mut self.world).await?;
        }
        Ok(())
    }
}
