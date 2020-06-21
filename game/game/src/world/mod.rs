mod game_world;
pub use self::game_world::*;
mod system;
pub use self::system::*;

pub mod test1;
pub mod test2;
pub mod test3;
pub mod test4;

use crate::assets::{AssetError, AssetIO, Url};
use crate::GameError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum WorldData {
    Test1(test1::Test1),
    Test2(test2::Test2),
    Test3(test3::Test3),
    Test4(test4::Test4),
}

impl WorldData {
    pub async fn from_url(assetio: &AssetIO, url: &Url) -> Result<WorldData, AssetError> {
        let world = assetio.download_binary(url).await?;
        let world = bincode::deserialize::<WorldData>(&world)
            .map_err(|err| AssetError::ContentLoad(format!("Binary stream error: {}", err)))?;
        Ok(world)
    }
}
