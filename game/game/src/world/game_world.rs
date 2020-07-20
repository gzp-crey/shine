use crate::{GameError, GameView};

pub trait GameLoadWorld: 'static + Send + Sync {
    type Source;

    fn build(source: Self::Source, game: &mut GameView) -> Result<Self, GameError>
    where
        Self: Sized;
}

pub trait GameUnloadWorld: 'static + Send + Sync {
    fn unload(&mut self, game: &mut GameView) -> Result<(), GameError>;
}
