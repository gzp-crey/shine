#[cfg(feature = "cook")]
mod source;
#[cfg(feature = "cook")]
pub use self::source::*;

mod technique;
mod test_pass;

use self::technique::Technique;
use self::test_pass::TestPass;
use crate::{
    app::{App, AppError, GameFuture, GameLifecycle, GameSource},
    assets::{AssetError, AssetIO, Url},
    World,
};
use serde::{Deserialize, Serialize};
use shine_ecs::scheduler::{IntoSystem, TaskGroup};
use std::error::Error as StdError;

#[derive(Debug, Serialize, Deserialize)]
pub enum Test1Type {
    Test1,
}

pub fn into_game_err<E: 'static + StdError>(error: E) -> AppError {
    AppError::game("test1", error)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Test1 {
    #[serde(rename = "type")]
    pub ty: Test1Type,
    pub pipeline: String,
}

impl Test1 {
    pub async fn load_into_app(app: &mut App, url: &Url) -> Result<(), AppError> {
        let game = {
            let assetio = app.world.resources.get::<AssetIO>().map_err(into_game_err)?;
            let data = assetio.download_binary(url).await.map_err(into_game_err)?;
            bincode::deserialize::<Test1>(&data)
                .map_err(|err| AssetError::load_failed(&url, err))
                .map_err(into_game_err)?
        };
        app.init_game(game).await
    }
}

impl GameSource for Test1 {
    fn build<'a>(self) -> Result<Box<dyn GameLifecycle>, AppError> {
        Ok(Box::new(self))
    }
}

impl GameLifecycle for Test1 {
    fn name(&self) -> String {
        "test1".to_owned()
    }

    fn create<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            world
                .resources
                .quick_insert(Technique::new(self.pipeline.clone()))
                .map_err(into_game_err)?;

            world.add_stage("render", TaskGroup::from_system(technique::render.into_system()));

            Ok(())
        })
    }

    fn destroy<'a>(&'a mut self, world: &'a mut World) -> GameFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            world.clear_stages();
            let _ = world.resources.remove::<Technique>();

            Ok(())
        })
    }
}
