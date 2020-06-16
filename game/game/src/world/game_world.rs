use crate::{GameError, GameView};
use std::any::Any;

pub trait GameWorldBuilder {
    type World: GameWorld;

    fn build(self, game: &mut GameView) -> Result<Self::World, GameError>;
}

pub trait GameWorld: 'static + Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError>;
}
