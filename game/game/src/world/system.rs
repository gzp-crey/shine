use crate::world::{GameWorld, GameWorldBuilder};
use crate::{GameError, GameView};

pub struct WorldLifecycle {
    world: Box<dyn GameWorld>,
}

pub trait WorldSystem {
    fn load_world<W: GameWorldBuilder>(&mut self, builder: W) -> Result<(), GameError>;
    fn unload_world(&mut self) -> Result<(), GameError>;
}

impl WorldSystem for GameView {
    fn load_world<W: GameWorldBuilder>(&mut self, builder: W) -> Result<(), GameError> {
        self.unload_world()?;
        let world = builder.build(self)?;
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
