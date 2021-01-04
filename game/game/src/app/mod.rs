mod error;
pub use self::error::*;
mod config;
pub use self::config::*;
mod game_lifecycle;
pub use self::game_lifecycle::*;
mod plugin;
pub use self::plugin::*;

use crate::World;
use std::collections::HashSet;

#[derive(Default)]
pub struct App {
    pub world: World,
    game_loader: Option<Box<dyn GameLifecycle>>,
    plugins: HashSet<String>,
}

impl App {
    pub fn plugins(&self) -> &HashSet<String> {
        &self.plugins
    }

    /// Initialize a plugin. Only one plugin can be initialized with the same name.
    pub async fn add_plugin<P: Plugin>(&mut self, plugin: P) -> Result<&mut Self, AppError> {
        let name = <P as Plugin>::name().to_string();
        if self.plugins.insert(name.clone()) {
            log::info!("Adding {} plugin", name);
            plugin.init(&mut self.world).await?;
            Ok(self)
        } else {
            log::warn!("Plugin {} already present", name);
            Err(AppError::PluginAlreadyPresent { plugin: name })
        }
    }

    pub async fn remove_plugin<P: Plugin>(&mut self) -> Result<&mut Self, AppError> {
        let name = <P as Plugin>::name().to_string();
        if self.plugins.remove(&name) {
            log::info!("Removing {} plugin", name);
            <P as Plugin>::deinit(&mut self.world).await?;
            Ok(self)
        } else {
            log::warn!("Plugin {} was not addoed or already removed", name);
            Ok(self)
        }
    }

    pub async fn init_game<S: GameSource>(&mut self, game: S) -> Result<(), AppError> {
        self.deinit_game().await?;
        let mut game_loader = game.build()?;
        log::info!("Creating game {}", game_loader.name());
        game_loader.create(&mut self.world).await?;
        self.game_loader = Some(game_loader);
        Ok(())
    }

    pub async fn deinit_game(&mut self) -> Result<(), AppError> {
        if let Some(mut game_loader) = self.game_loader.take() {
            log::info!("Destroying game {}", game_loader.name());
            game_loader.destroy(&mut self.world).await?;
        }
        Ok(())
    }

    pub async fn reload_game(&mut self) -> Result<(), AppError> {
        if let Some(game_loader) = &mut self.game_loader {
            log::info!("Reloading game {} - destroy", game_loader.name());
            game_loader.destroy(&mut self.world).await?;
            log::info!("Reloading game {} - create", game_loader.name());
            game_loader.create(&mut self.world).await?;
        }
        Ok(())
    }
}
