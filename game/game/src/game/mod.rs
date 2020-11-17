//pub mod test1;
//pub mod test2;
//pub mod test3;
//pub mod test4;
//pub mod test5;

use crate::{
    app::{AppError, GameLifecycle, GameSource},
    assets::{AssetError, AssetIO, Url},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Game {
    Empty,
    //Test1(test1::Test1),
    //Test2(test2::Test2),
    //Test3(test3::Test3),
    //Test4(test4::Test4),
    //Test5(test5::Test5),
}

impl Game {
    pub async fn from_url(assetio: &AssetIO, url: &Url) -> Result<Game, AssetError> {
        let world = assetio.download_binary(url).await?;
        let world = bincode::deserialize::<Game>(&world).map_err(|err| AssetError::load_failed(url.as_str(), err))?;
        Ok(world)
    }
}

impl GameSource for Game {
    fn build(self) -> Result<Box<dyn GameLifecycle>, AppError>
    where
        Self: Sized,
    {
        match self {
            Game::Test1(test1) => test1.build(),
        }
    }
}
