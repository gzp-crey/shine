pub mod test1;
pub mod test2;
pub mod test3;

use crate::assets::Url;
use crate::{GameError, GameRender};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum World {
    Test1(test1::Test1),
    Test2(test2::Test2),
    Test3(test3::Test3),
}

pub async fn unload_world(game: &mut GameRender) -> Result<(), GameError> {
    test1::unregister_test_scene(game).await?;
    test2::unregister_test_scene(game).await?;
    test3::unregister_test_scene(game).await?;
    game.gc_all();
    Ok(())
}

pub async fn load_world(url: &Url, game: &mut GameRender) -> Result<(), GameError> {
    unload_world(game).await?;

    let world = game
        .assetio
        .download_binary(url)
        .await
        .map_err(|err| GameError::Setup(format!("Failed to download world: {:?}", err)))?;
    let world = bincode::deserialize::<World>(&world)
        .map_err(|err| GameError::Setup(format!("Failed to parse world: {:?}", err)))?;

    match world {
        World::Test1(test) => test1::register_test_scene(test, game).await,
        World::Test2(test) => test2::register_test_scene(test, game).await,
        World::Test3(test) => test3::register_test_scene(test, game).await,
    }
}
