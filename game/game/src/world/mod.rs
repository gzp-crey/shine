pub mod test1;
pub mod test2;
pub mod test3;

use crate::{GameError, GameRender};

pub fn unregister(game: &mut GameRender) -> Result<(), GameError> {
    test1::unregister_test_scene(game)?;
    test2::unregister_test_scene(game)?;
    test3::unregister_test_scene(game)?;
    game.gc_all();
    Ok(())
}

pub fn register_test1(game: &mut GameRender) -> Result<(), GameError> {
    unregister(game)?;
    test1::register_test_scene(game)
}

pub fn register_test2(game: &mut GameRender) -> Result<(), GameError> {
    unregister(game)?;
    test2::register_test_scene(game)
}

pub fn register_test3(game: &mut GameRender) -> Result<(), GameError> {
    unregister(game)?;
    test3::register_test_scene(game)
}
