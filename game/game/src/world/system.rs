use crate::world::{GameLoadWorld, GameUnloadWorld};
use crate::{GameError, GameView};

pub struct WorldLifecycle {
    world: Box<dyn GameUnloadWorld>,
}

pub trait WorldSystem {
    fn load_world<W>(&mut self, source: W::Source) -> Result<(), GameError>
    where
        W: Sized + GameLoadWorld + GameUnloadWorld;

    fn unload_world(&mut self) -> Result<(), GameError>;
}

impl WorldSystem for GameView {
    fn load_world<W>(&mut self, source: W::Source) -> Result<(), GameError>
    where
        W: Sized + GameLoadWorld + GameUnloadWorld,
    {
        self.unload_world()?;
        let world = W::build(source, self)?;
        self.resources.insert(WorldLifecycle { world: Box::new(world) });
        Ok(())
    }

    fn unload_world(&mut self) -> Result<(), GameError> {
        if let Some(mut lifecycle) = self.resources.remove::<WorldLifecycle>() {
            lifecycle.world.unload(self)?;
        }
        Ok(())
    }
}
