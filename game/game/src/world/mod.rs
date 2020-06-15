pub mod test1;
pub mod test2;
pub mod test3;
pub mod test4;

use crate::assets::Url;
use crate::{GameError, GameView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum World {
    Test1(test1::Test1),
    Test2(test2::Test2),
    Test3(test3::Test3),
    Test4(test4::Test4),
}

pub fn unload_world(game: &mut GameView) -> Result<(), GameError> {
    test1::unregister_test_scene(game)?;
    test2::unregister_test_scene(game)?;
    test3::unregister_test_scene(game)?;
    test4::unregister_test_scene(game)?;
    game.gc_all();
    Ok(())
}

pub async fn load_world(url: &Url, game: &mut GameView) -> Result<(), GameError> {
    unload_world(game)?;

    let world = game
        .assetio
        .download_binary(url)
        .await
        .map_err(|err| GameError::Setup(format!("Failed to download world: {:?}", err)))?;
    let world = bincode::deserialize::<World>(&world)
        .map_err(|err| GameError::Setup(format!("Failed to parse world: {:?}", err)))?;

    match world {
        World::Test1(test) => test1::register_test_scene(test, game),
        World::Test2(test) => test2::register_test_scene(test, game),
        World::Test3(test) => test3::register_test_scene(test, game),
        World::Test4(test) => test4::register_test_scene(test, game),
    }
}
